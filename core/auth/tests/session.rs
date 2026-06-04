//! Session / refresh-rotation tests (spec AC18; ADR-0016 D2; risk register R5/R6).
//!
//! The centerpiece is `auth_refresh_rotation_replay_detected` — the **sole enforced control**
//! behind ADR-0016's accepted residual of indefinite refresh tokens (R6). The property
//! `prop_session_indefinite_until_admin_event` pins the other half: a session's liveness is
//! purely a function of its family status, never of time (the indefinite-session guarantee).

use boundless_auth::{
    evaluate_refresh, invalidation_for, FixedClock, InvalidationTrigger, RefreshPresentation,
    RefreshVerdict, Session, SessionFamilyStatus, UnixSeconds,
};
use boundless_domain::{MemberId, SessionFamilyId};
use proptest::prelude::*;
use uuid::Uuid;

fn session(expires_at: i64, status: SessionFamilyStatus) -> Session {
    Session {
        member_id: MemberId::from_uuid(Uuid::from_u128(42)),
        family_id: SessionFamilyId::from_uuid(Uuid::from_u128(7)),
        access_token_expires_at: UnixSeconds::new(expires_at),
        family_status: status,
    }
}

/// R6 (the named privacy-invariant test): a refresh credential that has been rotated away,
/// when replayed, is rejected **and revokes the entire family** — including the legitimate
/// current credential. This is what makes ADR-0016's indefinite sessions safe: a stolen
/// credential that the genuine client later rotates past cannot be quietly reused, and the
/// theft is self-detecting.
#[test]
fn auth_refresh_rotation_replay_detected() {
    // 1. A live family. The genuine client presents the current credential → rotate.
    let status = SessionFamilyStatus::Active;
    let v_rotate = evaluate_refresh(status, RefreshPresentation::Current);
    assert_eq!(v_rotate, RefreshVerdict::Rotate);
    assert!(v_rotate.is_accepted());
    assert_eq!(v_rotate.error_code(), None);
    // Rotation leaves a live family (the server advances to a fresh current credential).
    assert_eq!(
        v_rotate.resulting_family_status(status),
        SessionFamilyStatus::Active
    );

    // 2. The pre-rotation credential is now superseded. Someone replays it → kill the family.
    let v_replay = evaluate_refresh(status, RefreshPresentation::Superseded);
    assert_eq!(v_replay, RefreshVerdict::ReplayDetectedKillFamily);
    assert!(!v_replay.is_accepted());
    assert_eq!(v_replay.error_code(), Some("AUTH_REFRESH_REPLAY_DETECTED"));
    let after_replay = v_replay.resulting_family_status(status);
    assert_eq!(after_replay, SessionFamilyStatus::Revoked);

    // 3. The family is dead for everyone: even the legitimate current credential is now
    //    rejected — the genuine client must re-establish auth (no silent recovery).
    let v_after = evaluate_refresh(after_replay, RefreshPresentation::Current);
    assert_eq!(v_after, RefreshVerdict::Rejected);
    assert_eq!(v_after.error_code(), Some("AUTH_SESSION_INVALIDATED"));

    // 4. The kill is idempotent: replaying the still-superseded credential again, now that the
    //    family is already dead, is a plain Rejected — not a second "kill" verdict (no second
    //    replay alert) — and the family stays revoked (no resurrection). This is the only
    //    order-dependent seam: `Superseded` kills on an Active family but is rejected once dead.
    let v_second_replay = evaluate_refresh(after_replay, RefreshPresentation::Superseded);
    assert_eq!(v_second_replay, RefreshVerdict::Rejected);
    assert_eq!(
        v_second_replay.error_code(),
        Some("AUTH_SESSION_INVALIDATED")
    );
    assert_eq!(
        v_second_replay.resulting_family_status(after_replay),
        SessionFamilyStatus::Revoked
    );

    // A revoked family also reports a dead session at the model level.
    assert!(!session(1_000, after_replay).is_live());
}

#[test]
fn unknown_credential_is_rejected_without_killing_a_live_family() {
    let status = SessionFamilyStatus::Active;
    let v = evaluate_refresh(status, RefreshPresentation::Unknown);
    assert_eq!(v, RefreshVerdict::Rejected);
    // An unknown credential is not a replay — it must not revoke a healthy family.
    assert_eq!(
        v.resulting_family_status(status),
        SessionFamilyStatus::Active
    );
}

#[test]
fn revoked_family_rejects_every_presentation() {
    let status = SessionFamilyStatus::Revoked;
    for presented in [
        RefreshPresentation::Current,
        RefreshPresentation::Superseded,
        RefreshPresentation::Unknown,
    ] {
        let v = evaluate_refresh(status, presented);
        assert_eq!(v, RefreshVerdict::Rejected);
        // It stays revoked — a dead family never resurrects.
        assert_eq!(
            v.resulting_family_status(status),
            SessionFamilyStatus::Revoked
        );
    }
}

#[test]
fn needs_refresh_uses_injected_clock_with_skew() {
    let s = session(1_000, SessionFamilyStatus::Active);
    // No skew: refresh exactly at/after expiry.
    assert!(!s.needs_refresh(&FixedClock::at_secs(999), 0));
    assert!(s.needs_refresh(&FixedClock::at_secs(1_000), 0));
    // With a 60s lead, refresh kicks in 60s early (now + skew >= expires_at).
    assert!(s.needs_refresh(&FixedClock::at_secs(940), 60));
    assert!(!s.needs_refresh(&FixedClock::at_secs(939), 60));
    // A revoked family never silently refreshes — it routes to re-auth instead.
    let dead = session(1_000, SessionFamilyStatus::Revoked);
    assert!(!dead.needs_refresh(&FixedClock::at_secs(10_000), 0));
}

#[test]
fn needs_refresh_saturates_at_i64_max() {
    // A far-future clock plus the maximum skew must not panic and still reports due-for-refresh:
    // the saturating add clamps at i64::MAX, which is >= any expiry instant.
    let s = session(i64::MAX, SessionFamilyStatus::Active);
    assert!(s.needs_refresh(&FixedClock::at_secs(i64::MAX), u32::MAX));
}

/// Map a 0..4 index to "no event" or one of the three invalidation triggers, so the property
/// can range over "an admin-mediated event happened, or not."
fn trigger_for_index(i: u8) -> Option<InvalidationTrigger> {
    match i {
        1 => Some(InvalidationTrigger::AdminRevokeOrLogout),
        2 => Some(InvalidationTrigger::NewDeviceReonboarding),
        3 => Some(InvalidationTrigger::AccountDeletion),
        _ => None,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1024, .. ProptestConfig::default() })]

    /// AC18: a session stays live indefinitely until — and only until — an admin-mediated
    /// invalidation event. Liveness is invariant to the clock (any `now`, any access-token
    /// expiry, any refresh skew): time alone never ends a session. An event ends it because
    /// every [`InvalidationTrigger`] revokes the family; absent an event it remains live.
    #[test]
    fn prop_session_indefinite_until_admin_event(
        now in any::<i64>(),
        expires_at in any::<i64>(),
        skew_secs in 0u32..86_400,
        event_index in 0u8..4,
    ) {
        let event = trigger_for_index(event_index);

        // An event revokes the family (every trigger ends the session); none keeps it active.
        let status = match event {
            Some(t) => {
                prop_assert!(invalidation_for(t).ends_session);
                SessionFamilyStatus::Revoked
            }
            None => SessionFamilyStatus::Active,
        };

        let s = session(expires_at, status);

        // Liveness depends ONLY on whether an admin event occurred — never on time.
        prop_assert_eq!(s.is_live(), event.is_none());

        // The refresh decision never changes liveness, is gated on liveness, and — for a live
        // session — is exactly the injected-clock predicate (an independent, saturating oracle,
        // matching `UnixSeconds::saturating_add_secs`). A dead family must never refresh.
        let clock = FixedClock::at_secs(now);
        let refresh = s.needs_refresh(&clock, skew_secs);
        prop_assert_eq!(s.is_live(), event.is_none());
        prop_assert!(!refresh || s.is_live(), "a dead family must never silently refresh");
        if s.is_live() {
            prop_assert_eq!(refresh, now.saturating_add(i64::from(skew_secs)) >= expires_at);
        } else {
            prop_assert!(!refresh);
        }
    }
}
