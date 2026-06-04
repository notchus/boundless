//! `/api/auth/recovery/rebind` — Driver self-serve device replacement (AC19): re-bind, old token
//! invalidated, fresh code issued; Riders have no self-serve path.

mod common;

use boundless_auth::RecoveryCodeVerdict;
use boundless_domain::{AppVersion, ClientVersion, DeviceToken, Platform, RecoveryCode, Role};
use boundless_server_core::{normalize_phone, AlertKind, RecoveryOutcome, RecoveryRequest};
use common::*;

fn recovery_req(phone: &str, code: &str, reported: ClientVersion) -> RecoveryRequest {
    RecoveryRequest {
        phone: normalize_phone(phone).expect("valid E.164"),
        code: RecoveryCode::new(code),
        reported,
        device_token: DeviceToken::new("new-device-token"),
    }
}

fn driver_with_recovery() -> MemStore {
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Driver]);
    store.add_recovery(member_id(1), "REC-GOOD");
    // A prior device the recovery must replace.
    store.add_device(member_id(1), Platform::Ios, AppVersion::new(1, 1, 0));
    store
}

#[test]
fn ac19_driver_rebind_succeeds_invalidates_old_and_issues_fresh_code() {
    use boundless_auth::DeviceBinding;
    let prior = DeviceBinding::new(member_id(1), Platform::Ios, AppVersion::new(1, 1, 0));
    let mut svc = service(driver_with_recovery(), 1_000);

    let resp = svc.recovery_rebind(recovery_req("+15550000001", "REC-GOOD", ios_current()));

    // Borrow (don't move) the outcome so `error_code()` can still read `resp` afterwards.
    match &resp.outcome {
        RecoveryOutcome::Rebound {
            fresh_recovery_code,
            ..
        } => {
            // A fresh Recovery Code is issued (rotated on use) for the Driver to capture.
            assert!(!fresh_recovery_code.expose_secret().is_empty());
        }
        _ => panic!("expected Rebound for a valid Driver recovery"),
    }
    assert_eq!(resp.error_code(), None);
    // AC7: both version fields present on the recovery response too.
    assert_eq!(resp.version.min, AppVersion::new(1, 0, 0));
    assert_eq!(resp.version.recommended, AppVersion::new(1, 2, 0));
    assert!(svc.store.is_invalidated(&prior));
    assert_eq!(svc.store.active_device_count(member_id(1)), 1);
}

#[test]
fn ac19_rider_has_no_self_serve_recovery() {
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
    // Even if a recovery code somehow existed, a Rider is refused before any secret comparison.
    store.add_recovery(member_id(1), "REC-GOOD");
    let mut svc = service(store, 1_000);

    let resp = svc.recovery_rebind(recovery_req("+15550000001", "REC-GOOD", ios_current()));

    assert!(matches!(
        resp.outcome,
        RecoveryOutcome::Rejected(RecoveryCodeVerdict::NotAvailable)
    ));
    assert_eq!(resp.error_code(), Some("AUTH_RECOVERY_NOT_AVAILABLE"));
}

#[test]
fn ac19_reused_recovery_code_is_rejected() {
    let mut svc = service(driver_with_recovery(), 1_000);

    let first = svc.recovery_rebind(recovery_req("+15550000001", "REC-GOOD", ios_current()));
    assert!(matches!(first.outcome, RecoveryOutcome::Rebound { .. }));

    // The code was rotated on use; presenting the old one again no longer matches.
    let second = svc.recovery_rebind(recovery_req("+15550000001", "REC-GOOD", ios_current()));
    assert!(matches!(
        second.outcome,
        RecoveryOutcome::Rejected(RecoveryCodeVerdict::Invalid)
    ));
}

#[test]
fn recovery_wrong_code_is_invalid() {
    let mut svc = service(driver_with_recovery(), 1_000);
    let resp = svc.recovery_rebind(recovery_req("+15550000001", "REC-WRONG", ios_current()));
    assert!(matches!(
        resp.outcome,
        RecoveryOutcome::Rejected(RecoveryCodeVerdict::Invalid)
    ));
}

#[test]
fn recovery_no_live_code_is_invalid() {
    // A Driver who lost their Recovery Code (no live row) gets `Invalid` and falls back to the
    // Admin path — not a different shape that would reveal the code's absence.
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Driver]);
    // No `add_recovery` — the Driver has no live code.
    let mut svc = service(store, 1_000);
    let resp = svc.recovery_rebind(recovery_req("+15550000001", "REC-GOOD", ios_current()));
    assert!(matches!(
        resp.outcome,
        RecoveryOutcome::Rejected(RecoveryCodeVerdict::Invalid)
    ));
}

#[test]
fn recovery_below_min_degrades() {
    let mut svc = service(driver_with_recovery(), 1_000);
    let resp = svc.recovery_rebind(recovery_req("+15550000001", "REC-GOOD", ios_below_min()));
    assert!(matches!(resp.outcome, RecoveryOutcome::BelowMinVersion));
    assert_eq!(resp.error_code(), Some("AUTH_BELOW_MIN_VERSION"));
    // The member is known, so the below-min admin alert fires once — parity with the other endpoints.
    assert_eq!(svc.alerts.count_kind(AlertKind::BelowMinVersion), 1);
}
