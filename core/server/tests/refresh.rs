//! `/api/auth/refresh` — silent rotation (AC18), replay detection → family kill + AC15 alert,
//! and the uniform `Invalidated` outcome for unknown/revoked/replay (no lineage leak,
//! carry-forward (a)/(e)).

mod common;

use boundless_auth::{RefreshVerdict, SessionFamilyStatus};
use boundless_domain::{ClientVersion, RefreshToken};
use boundless_server_core::{AlertKind, RefreshOutcome, RefreshRequest, SourceKey};
use common::*;

const FAR_FUTURE: i64 = 10_000;

fn refresh_req(token: &str, reported: ClientVersion) -> RefreshRequest {
    RefreshRequest {
        presented: RefreshToken::new(token),
        reported,
        source: SourceKey(7),
    }
}

#[test]
fn ac18_rotate_current_credential() {
    let mut store = MemStore::new();
    store.add_family(
        member_id(1),
        "R-current",
        &[],
        SessionFamilyStatus::Active,
        FAR_FUTURE,
    );
    let mut svc = service(store, 1_000);

    let resp = svc.refresh_ok(refresh_req("R-current", ios_current()));

    assert!(matches!(resp.outcome, RefreshOutcome::Rotated(_)));
    assert_eq!(resp.server_verdict, Some(RefreshVerdict::Rotate));
    // AC7: both version fields present on the refresh response too.
    assert_eq!(resp.version.min, boundless_domain::AppVersion::new(1, 0, 0));
    assert_eq!(
        resp.version.recommended,
        boundless_domain::AppVersion::new(1, 2, 0)
    );
}

#[test]
fn auth_refresh_rotation_replay_detected() {
    let mut store = MemStore::new();
    // A family whose current credential is `R-new`; `R-old` was rotated away (superseded).
    let fam = store.add_family(
        member_id(1),
        "R-new",
        &["R-old"],
        SessionFamilyStatus::Active,
        FAR_FUTURE,
    );
    let mut svc = service(store, 1_000);

    // Replaying the rotated-away credential kills the whole family (R6) and alerts the admin (AC15).
    let replay = svc.refresh_ok(refresh_req("R-old", ios_current()));
    assert!(matches!(replay.outcome, RefreshOutcome::Invalidated));
    assert_eq!(
        replay.server_verdict,
        Some(RefreshVerdict::ReplayDetectedKillFamily)
    );
    assert_eq!(
        svc.store.family_status(fam),
        Some(SessionFamilyStatus::Revoked)
    );
    assert_eq!(svc.alerts.count_kind(AlertKind::SessionInvalidated), 1);

    // The legitimate current credential is now rejected too — the family is dead (AC18).
    let legit = svc.refresh_ok(refresh_req("R-new", ios_current()));
    assert!(matches!(legit.outcome, RefreshOutcome::Invalidated));
    assert_eq!(legit.server_verdict, Some(RefreshVerdict::Rejected));
}

#[test]
fn refresh_unknown_credential_is_rejected_no_alert_no_change() {
    let mut store = MemStore::new();
    let fam = store.add_family(
        member_id(1),
        "R-current",
        &[],
        SessionFamilyStatus::Active,
        FAR_FUTURE,
    );
    let mut svc = service(store, 1_000);

    let resp = svc.refresh_ok(refresh_req("R-nobody", ios_current()));

    assert!(matches!(resp.outcome, RefreshOutcome::Invalidated));
    assert_eq!(resp.server_verdict, Some(RefreshVerdict::Rejected));
    // An unknown credential touches nothing and raises no "rider needs help" alert.
    assert_eq!(
        svc.store.family_status(fam),
        Some(SessionFamilyStatus::Active)
    );
    assert!(svc.alerts.alerts.is_empty());
}

#[test]
fn refresh_reject_shape_identical_for_unknown_revoked_and_replay() {
    // Unknown credential.
    let mut svc1 = service(MemStore::new(), 1_000);
    let unknown = svc1.refresh_ok(refresh_req("R-nobody", ios_current()));

    // Already-revoked family.
    let mut store2 = MemStore::new();
    store2.add_family(
        member_id(1),
        "R-revoked",
        &[],
        SessionFamilyStatus::Revoked,
        FAR_FUTURE,
    );
    let mut svc2 = service(store2, 1_000);
    let revoked = svc2.refresh_ok(refresh_req("R-revoked", ios_current()));

    // Replay (superseded credential of a live family).
    let mut store3 = MemStore::new();
    store3.add_family(
        member_id(1),
        "R-new",
        &["R-old"],
        SessionFamilyStatus::Active,
        FAR_FUTURE,
    );
    let mut svc3 = service(store3, 1_000);
    let replay = svc3.refresh_ok(refresh_req("R-old", ios_current()));

    // All three present the SAME client-facing outcome — `Invalidated` — so the client cannot tell
    // "once-valid" from "unknown" from "revoked" (no lineage-existence leak).
    assert!(matches!(unknown.outcome, RefreshOutcome::Invalidated));
    assert!(matches!(revoked.outcome, RefreshOutcome::Invalidated));
    assert!(matches!(replay.outcome, RefreshOutcome::Invalidated));
}

#[test]
fn refresh_replay_twice_same_day_alerts_once() {
    let mut store = MemStore::new();
    let fam = store.add_family(
        member_id(1),
        "R-new",
        &["R-old"],
        SessionFamilyStatus::Active,
        FAR_FUTURE,
    );
    let mut svc = service(store, 1_000);

    let _ = svc.refresh_ok(refresh_req("R-old", ios_current()));
    let _ = svc.refresh_ok(refresh_req("R-old", ios_current())); // a second replay the same day
                                                                 // The family is killed on the first; the alert is deduped to one per member per day (AC15).
    assert_eq!(
        svc.store.family_status(fam),
        Some(SessionFamilyStatus::Revoked)
    );
    assert_eq!(svc.alerts.count_kind(AlertKind::SessionInvalidated), 1);
}

#[test]
fn refresh_replay_below_min_degrades_and_leaves_family_intact() {
    // A thief on an old build cannot trigger a family-kill: the version gate precedes the refresh
    // policy, so a replayed credential from a below-min client degrades without revoking the
    // family — and a legitimate holder on an old build is never locked out by it.
    let mut store = MemStore::new();
    let fam = store.add_family(
        member_id(1),
        "R-new",
        &["R-old"],
        SessionFamilyStatus::Active,
        FAR_FUTURE,
    );
    let mut svc = service(store, 1_000);

    let resp = svc.refresh_ok(refresh_req("R-old", ios_below_min()));
    assert!(matches!(resp.outcome, RefreshOutcome::BelowMinVersion));
    assert_eq!(resp.server_verdict, None);
    assert_eq!(
        svc.store.family_status(fam),
        Some(SessionFamilyStatus::Active)
    );
    assert_eq!(svc.alerts.count_kind(AlertKind::SessionInvalidated), 0);
}

#[test]
fn refresh_below_min_degrades_without_touching_session() {
    let mut store = MemStore::new();
    let fam = store.add_family(
        member_id(1),
        "R-current",
        &[],
        SessionFamilyStatus::Active,
        FAR_FUTURE,
    );
    let mut svc = service(store, 1_000);

    let resp = svc.refresh_ok(refresh_req("R-current", ios_below_min()));

    assert!(matches!(resp.outcome, RefreshOutcome::BelowMinVersion));
    assert_eq!(resp.server_verdict, None); // the refresh policy never ran
                                           // The session is untouched — the app is merely too old; once it updates, refresh resumes.
    assert_eq!(
        svc.store.family_status(fam),
        Some(SessionFamilyStatus::Active)
    );
    assert_eq!(svc.alerts.count_kind(AlertKind::BelowMinVersion), 1);
}
