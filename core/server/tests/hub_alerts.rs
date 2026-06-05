//! The `GroupHub` counters + admin-alert dedup (§10-E): one alert per member per day (AC8/AC15),
//! the attempt window, and the PII-free alert payload shape (R12 / I8 / P2).

mod common;

use boundless_auth::{FixedClock, SignInResult, UnixSeconds};
use boundless_domain::{AppVersion, Role};
use boundless_server_core::{
    AdminAlert, AlertKind, GroupHubState, SignInRequest, SourceKey, CODE_ATTEMPT_WINDOW_SECS,
};
use common::*;

const DAY: i64 = 24 * 60 * 60;

fn signin_below_min(svc: &mut TestService, member_phone: &str) -> SignInResult {
    svc.sign_in_ok(SignInRequest {
        phone: boundless_server_core::normalize_phone(member_phone).expect("valid E.164"),
        reported: ios_below_min(),
    })
    .result
}

#[test]
fn ac8_below_min_emits_one_alert_per_member_per_day() {
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
    let mut svc = service(store, DAY * 100 + 5); // some arbitrary day, mid-day

    // Two below-min sign-ins for the same member the same day → exactly one alert (deduped).
    assert_eq!(
        signin_below_min(&mut svc, "+15550000001"),
        SignInResult::BelowMinVersion
    );
    assert_eq!(
        signin_below_min(&mut svc, "+15550000001"),
        SignInResult::BelowMinVersion
    );
    assert_eq!(svc.alerts.count_kind(AlertKind::BelowMinVersion), 1);

    // The next day, the same member alerts again.
    svc.clock = FixedClock::at_secs(DAY * 101 + 5);
    assert_eq!(
        signin_below_min(&mut svc, "+15550000001"),
        SignInResult::BelowMinVersion
    );
    assert_eq!(svc.alerts.count_kind(AlertKind::BelowMinVersion), 2);
}

#[test]
fn ac15_invalidated_rider_alert_once_per_day() {
    let mut svc = service(MemStore::new(), DAY * 50 + 10);
    let m = member_id(7);

    svc.note_session_invalidated(m);
    svc.note_session_invalidated(m); // same day → deduped
    assert_eq!(svc.alerts.count_kind(AlertKind::SessionInvalidated), 1);

    svc.clock = FixedClock::at_secs(DAY * 51 + 10);
    svc.note_session_invalidated(m);
    assert_eq!(svc.alerts.count_kind(AlertKind::SessionInvalidated), 2);
}

#[test]
fn alert_dedup_is_independent_across_members_and_kinds() {
    let mut hub = GroupHubState::new();
    let now = UnixSeconds::new(0);

    // Distinct members: each alerts independently.
    assert!(hub.should_alert(member_id(1), AlertKind::SessionInvalidated, now));
    assert!(hub.should_alert(member_id(2), AlertKind::SessionInvalidated, now));
    // Same member, same kind, same day: suppressed.
    assert!(!hub.should_alert(member_id(1), AlertKind::SessionInvalidated, now));
    // Same member, DIFFERENT kind: independent.
    assert!(hub.should_alert(member_id(1), AlertKind::BelowMinVersion, now));
    // The next day re-opens the window.
    assert!(hub.should_alert(
        member_id(1),
        AlertKind::SessionInvalidated,
        UnixSeconds::new(DAY)
    ));
}

#[test]
fn code_attempt_window_counts_prior_and_resets_each_window() {
    let mut hub = GroupHubState::new();
    let m = member_id(1);

    // Prior counts are returned (0, 1, 2, …) within one window.
    assert_eq!(hub.register_code_attempt(m, UnixSeconds::new(0)), 0);
    assert_eq!(hub.register_code_attempt(m, UnixSeconds::new(10)), 1);
    assert_eq!(hub.register_code_attempt(m, UnixSeconds::new(20)), 2);

    // A new 15-minute window resets the count.
    let next_window = UnixSeconds::new(CODE_ATTEMPT_WINDOW_SECS + 1);
    assert_eq!(hub.register_code_attempt(m, next_window), 0);
}

#[test]
fn refresh_rejections_count_per_source_and_reset_each_window() {
    let mut hub = GroupHubState::new();
    let a = SourceKey(1);
    let b = SourceKey(2);

    // Each rejection from a source increments its own count (the value returned INCLUDES this one).
    assert_eq!(hub.note_refresh_rejection(a, UnixSeconds::new(0)), 1);
    assert_eq!(hub.note_refresh_rejection(a, UnixSeconds::new(5)), 2);
    // A different source is independent.
    assert_eq!(hub.note_refresh_rejection(b, UnixSeconds::new(5)), 1);
    // A new 15-minute window resets the per-source count.
    assert_eq!(
        hub.note_refresh_rejection(a, UnixSeconds::new(CODE_ATTEMPT_WINDOW_SECS + 1)),
        1
    );
}

#[test]
fn admin_alert_payload_is_pii_free() {
    // Every variant serializes to only an opaque member id (+ a version string) — no phone, token,
    // or other secret. The tainted types are not `Serialize`, so one could not be added here.
    let cases = [
        (
            AdminAlert::BelowMinVersion {
                member: member_id(1),
                reported_version: AppVersion::new(0, 9, 0),
            },
            r#"{"kind":"below_min_version","member":"00000000-0000-0000-0000-000000000001","reported_version":"0.9.0"}"#,
        ),
        (
            AdminAlert::SessionInvalidated {
                member: member_id(2),
            },
            r#"{"kind":"session_invalidated","member":"00000000-0000-0000-0000-000000000002"}"#,
        ),
        (
            AdminAlert::OnboardingCodeLocked {
                member: member_id(3),
            },
            r#"{"kind":"onboarding_code_locked","member":"00000000-0000-0000-0000-000000000003"}"#,
        ),
        (
            AdminAlert::NotificationsNotEnabled {
                member: member_id(4),
            },
            r#"{"kind":"notifications_not_enabled","member":"00000000-0000-0000-0000-000000000004"}"#,
        ),
    ];
    for (alert, expected) in cases {
        let json = serde_json::to_string(&alert).expect("alert serializes");
        assert_eq!(json, expected);
        for forbidden in ["phone", "token", "secret", "redacted", "+1555"] {
            assert!(
                !json.contains(forbidden),
                "alert payload leaked `{forbidden}`: {json}"
            );
        }
    }
}
