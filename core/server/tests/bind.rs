//! `/api/auth/bind-device` — Onboarding-Code lifecycle (AC17), device-token binding +
//! prior-token invalidation (AC4/I4, F5 "invalidate all"), rate-limit lock + alert (AC17/R4),
//! atomic consume (carry-forward (a)), silent device-token invalidation (carry-forward (e)), and
//! the AC14 notification-decline flag.

mod common;

use boundless_auth::{DeviceBinding, OnboardingCodeVerdict};
use boundless_domain::{AppVersion, ClientVersion, DeviceToken, OnboardingCode, Platform, Role};
use boundless_server_core::{normalize_phone, AlertKind, BindOutcome, BindRequest};
use common::*;

const FAR_FUTURE: i64 = 10_000;

fn bind_req(phone: &str, code: &str, reported: ClientVersion) -> BindRequest {
    BindRequest {
        phone: normalize_phone(phone).expect("valid E.164"),
        code: OnboardingCode::new(code),
        reported,
        device_token: DeviceToken::new("dev-token"),
    }
}

fn rider_with_code() -> MemStore {
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
    store.add_onboarding(member_id(1), "ONB-GOOD", FAR_FUTURE, 5);
    store
}

#[test]
fn ac17_valid_code_binds_and_carries_version() {
    let mut svc = service(rider_with_code(), 1_000);
    let resp = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()));

    assert!(matches!(resp.outcome, BindOutcome::Bound(_)));
    assert_eq!(resp.error_code(), None);
    // AC7: every /api/auth/* response carries BOTH min and recommended.
    assert_eq!(resp.version.min, AppVersion::new(1, 0, 0));
    assert_eq!(resp.version.recommended, AppVersion::new(1, 2, 0));
    assert_eq!(svc.store.active_device_count(member_id(1)), 1);
}

#[test]
fn ac17_wrong_code_is_invalid() {
    let mut svc = service(rider_with_code(), 1_000);
    let resp = svc.bind_device(bind_req("+15550000001", "ONB-WRONG", ios_current()));
    assert!(matches!(
        resp.outcome,
        BindOutcome::Failed(OnboardingCodeVerdict::Invalid)
    ));
    assert_eq!(resp.error_code(), Some("AUTH_ONBOARDING_CODE_INVALID"));
}

#[test]
fn ac17_expired_code_is_rejected_server_time() {
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
    store.add_onboarding(member_id(1), "ONB-GOOD", 500, 5); // expires_at = 500
    let mut svc = service(store, 1_000); // server clock = 1_000 > 500 → expired
    let resp = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()));
    assert!(matches!(
        resp.outcome,
        BindOutcome::Failed(OnboardingCodeVerdict::Expired)
    ));
}

#[test]
fn ac17_unknown_member_is_invalid_no_existence_leak() {
    let mut svc = service(rider_with_code(), 1_000);
    // A phone with no member: same `Invalid` shape as a bad code — no existence signal.
    let resp = svc.bind_device(bind_req("+15559999999", "ONB-GOOD", ios_current()));
    assert!(matches!(
        resp.outcome,
        BindOutcome::Failed(OnboardingCodeVerdict::Invalid)
    ));
}

#[test]
fn bind_atomic_consume_no_double_bind() {
    let mut svc = service(rider_with_code(), 1_000);

    let first = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()));
    let second = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()));

    // Exactly one bind; the second presentation of the now-consumed code is `Consumed`, never a
    // second session/device (carry-forward (a)).
    assert!(matches!(first.outcome, BindOutcome::Bound(_)));
    assert!(matches!(
        second.outcome,
        BindOutcome::Failed(OnboardingCodeVerdict::Consumed)
    ));
    assert_eq!(svc.store.active_device_count(member_id(1)), 1);
}

#[test]
fn ac4_reonboarding_invalidates_prior_device_token_silently() {
    let mut store = rider_with_code();
    // A prior device on an older app version.
    store.add_device(member_id(1), Platform::Ios, AppVersion::new(1, 1, 0));
    let prior = DeviceBinding::new(member_id(1), Platform::Ios, AppVersion::new(1, 1, 0));
    let mut svc = service(store, 1_000);

    let resp = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()));

    assert!(matches!(resp.outcome, BindOutcome::Bound(_)));
    // The prior token is invalidated, and the new one is the only active binding (AC4/I4)...
    assert!(svc.store.is_invalidated(&prior));
    assert_eq!(svc.store.active_device_count(member_id(1)), 1);
    // ...and the device-token invalidation is SILENT — it never surfaces in the client response
    // (carry-forward (e)).
    assert_eq!(resp.error_code(), None);
}

#[test]
fn reonboarding_with_multiple_prior_bindings_invalidates_all() {
    let mut store = rider_with_code();
    // Two prior devices (phone on an old version + an iPad): re-onboarding must clear ALL of them
    // (decision: single active device per member; F5 "invalidate all" — no stale token survives).
    store.add_device(member_id(1), Platform::Ios, AppVersion::new(1, 1, 0));
    store.add_device(member_id(1), Platform::IpadOs, AppVersion::new(1, 0, 0));
    let prior_phone = DeviceBinding::new(member_id(1), Platform::Ios, AppVersion::new(1, 1, 0));
    let prior_ipad = DeviceBinding::new(member_id(1), Platform::IpadOs, AppVersion::new(1, 0, 0));
    let mut svc = service(store, 1_000);

    let resp = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()));

    assert!(matches!(resp.outcome, BindOutcome::Bound(_)));
    assert!(svc.store.is_invalidated(&prior_phone));
    assert!(svc.store.is_invalidated(&prior_ipad));
    assert_eq!(svc.store.active_device_count(member_id(1)), 1);
}

#[test]
fn ac17_rate_limit_locks_and_alerts_after_max_attempts() {
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
    store.add_onboarding(member_id(1), "ONB-GOOD", FAR_FUTURE, 5); // 5 attempts allowed
    let mut svc = service(store, 1_000);

    // Five wrong attempts (prior counts 0..4) are `Invalid`; they keep the code live.
    for _ in 0..5 {
        let r = svc.bind_device(bind_req("+15550000001", "ONB-WRONG", ios_current()));
        assert!(matches!(
            r.outcome,
            BindOutcome::Failed(OnboardingCodeVerdict::Invalid)
        ));
    }
    // The sixth (prior count 5 ≥ max) locks, before the code is even compared.
    let locked = svc.bind_device(bind_req("+15550000001", "ONB-WRONG", ios_current()));
    assert!(matches!(
        locked.outcome,
        BindOutcome::Failed(OnboardingCodeVerdict::RateLimited)
    ));
    assert_eq!(svc.store.active_device_count(member_id(1)), 0);
    assert_eq!(svc.alerts.count_kind(AlertKind::OnboardingCodeLocked), 1);
}

#[test]
fn bind_below_min_degrades_without_binding() {
    let mut svc = service(rider_with_code(), 1_000);
    let resp = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_below_min()));
    assert!(matches!(resp.outcome, BindOutcome::BelowMinVersion));
    assert_eq!(resp.error_code(), Some("AUTH_BELOW_MIN_VERSION"));
    assert_eq!(svc.store.active_device_count(member_id(1)), 0);
    assert_eq!(svc.alerts.count_kind(AlertKind::BelowMinVersion), 1);
}

#[test]
fn ac17_regenerated_code_supersedes_prior() {
    // Regenerate-invalidates-prior (AC17): the admin re-issues a code, replacing the live one.
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
    store.add_onboarding(member_id(1), "ONB-OLD", FAR_FUTURE, 5);
    store.add_onboarding(member_id(1), "ONB-NEW", FAR_FUTURE, 5); // regenerate → only NEW is live
    let mut svc = service(store, 1_000);

    // The old (superseded) code no longer binds...
    let old = svc.bind_device(bind_req("+15550000001", "ONB-OLD", ios_current()));
    assert!(matches!(
        old.outcome,
        BindOutcome::Failed(OnboardingCodeVerdict::Invalid)
    ));
    // ...the new one does.
    let new = svc.bind_device(bind_req("+15550000001", "ONB-NEW", ios_current()));
    assert!(matches!(new.outcome, BindOutcome::Bound(_)));
}

#[test]
fn ac17_correct_code_on_last_allowed_attempt_binds() {
    let mut svc = service(rider_with_code(), 1_000); // max_attempts = 5
                                                     // Four wrong attempts (prior counts 0..3) keep the code live.
    for _ in 0..4 {
        let _ = svc.bind_device(bind_req("+15550000001", "ONB-WRONG", ios_current()));
    }
    // The fifth attempt (prior count 4 < 5) with the CORRECT code still binds.
    let resp = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()));
    assert!(matches!(resp.outcome, BindOutcome::Bound(_)));
}

#[test]
fn ac17_correct_code_after_lock_is_still_rate_limited() {
    let mut svc = service(rider_with_code(), 1_000); // max_attempts = 5
                                                     // Five wrong attempts (prior counts 0..4) exhaust the window.
    for _ in 0..5 {
        let _ = svc.bind_device(bind_req("+15550000001", "ONB-WRONG", ios_current()));
    }
    // The sixth (prior count 5 ≥ max) is locked even though the code is CORRECT — the lock beats
    // the secret, which is never compared once locked.
    let resp = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()));
    assert!(matches!(
        resp.outcome,
        BindOutcome::Failed(OnboardingCodeVerdict::RateLimited)
    ));
    assert_eq!(svc.store.active_device_count(member_id(1)), 0);
}

#[test]
fn ac17_rate_limit_resets_next_window_then_binds() {
    use boundless_server_core::CODE_ATTEMPT_WINDOW_SECS;
    let mut svc = service(rider_with_code(), 1_000); // max_attempts = 5, TTL far future
    for _ in 0..6 {
        let _ = svc.bind_device(bind_req("+15550000001", "ONB-WRONG", ios_current()));
    }
    // The lock is a *window*, not permanent: in the next window the correct code binds.
    svc.clock = boundless_auth::FixedClock::at_secs(1_000 + CODE_ATTEMPT_WINDOW_SECS + 1);
    let resp = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()));
    assert!(matches!(resp.outcome, BindOutcome::Bound(_)));
}

#[test]
fn ac17_code_expired_exactly_at_ttl_instant() {
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
    store.add_onboarding(member_id(1), "ONB-GOOD", 1_000, 5); // expires_at == now
    let mut svc = service(store, 1_000);
    let resp = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()));
    // The TTL boundary is exclusive (`now >= expires_at` ⇒ expired).
    assert!(matches!(
        resp.outcome,
        BindOutcome::Failed(OnboardingCodeVerdict::Expired)
    ));
}

#[test]
fn ac17_code_live_one_second_before_ttl_binds() {
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
    store.add_onboarding(member_id(1), "ONB-GOOD", 1_001, 5); // expires_at = now + 1
    let mut svc = service(store, 1_000);
    let resp = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()));
    assert!(matches!(resp.outcome, BindOutcome::Bound(_)));
}

#[test]
fn rebinding_same_device_tuple_keeps_one_active_binding() {
    // Re-onboarding the identical (member, platform, app_version) tuple replaces the token rather
    // than accumulating a duplicate active binding (the I4 composite-key upsert).
    let mut store = rider_with_code();
    store.add_device(member_id(1), Platform::Ios, AppVersion::new(1, 2, 0)); // same tuple as ios_current
    let mut svc = service(store, 1_000);
    let resp = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_current()));
    assert!(matches!(resp.outcome, BindOutcome::Bound(_)));
    assert_eq!(svc.store.active_device_count(member_id(1)), 1);
}

#[test]
fn bind_below_min_unknown_phone_is_identical_and_silent() {
    // The below-min admin alert is keyed by member; an unknown phone has no member to help, so it
    // fires no alert — and the response is identical to a known-phone below-min (no existence leak,
    // since the alert goes to the admin, never the client).
    let mut svc = service(rider_with_code(), 1_000);
    let known = svc.bind_device(bind_req("+15550000001", "ONB-GOOD", ios_below_min()));
    let unknown = svc.bind_device(bind_req("+15559999999", "ONB-GOOD", ios_below_min()));
    assert!(matches!(known.outcome, BindOutcome::BelowMinVersion));
    assert!(matches!(unknown.outcome, BindOutcome::BelowMinVersion));
    // Only the real member's below-min alerts; the unknown one is silent.
    assert_eq!(svc.alerts.count_kind(AlertKind::BelowMinVersion), 1);
}

#[test]
fn ac14_notification_decline_records_flag_and_advances() {
    let mut svc = service(rider_with_code(), 1_000);

    // Declined → flagged (returns true) + a single non-PII admin flag; the flow advances anyway.
    assert!(svc.record_notification_decision(member_id(1), false));
    assert_eq!(svc.alerts.count_kind(AlertKind::NotificationsNotEnabled), 1);

    // Granted for the SAME member → no flag, and no new alert (a grant never alerts).
    assert!(!svc.record_notification_decision(member_id(1), true));
    assert_eq!(svc.alerts.count_kind(AlertKind::NotificationsNotEnabled), 1);

    // A second decline the same day is deduped (still one alert).
    assert!(svc.record_notification_decision(member_id(1), false));
    assert_eq!(svc.alerts.count_kind(AlertKind::NotificationsNotEnabled), 1);
}
