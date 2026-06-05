//! `/api/auth/signin` — version handshake (AC7), below-min degradation + alert (AC8), and the
//! no-existence-leak response shape (carry-forward (b)).

mod common;

use boundless_auth::SignInResult;
use boundless_domain::{AppVersion, ClientVersion, Role};
use boundless_server_core::{normalize_phone, AlertKind, SignInRequest};
use common::*;

fn signin_req(phone: &str, reported: ClientVersion) -> SignInRequest {
    SignInRequest {
        phone: normalize_phone(phone).expect("valid E.164"),
        reported,
    }
}

#[test]
fn ac7_signin_response_carries_min_and_recommended_version() {
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
    let mut svc = service(store, 1_000);

    let resp = svc.sign_in_ok(signin_req("+15550000001", ios_current()));

    assert_eq!(resp.version.min, AppVersion::new(1, 0, 0));
    assert_eq!(resp.version.recommended, AppVersion::new(1, 2, 0));
    assert_eq!(resp.result, SignInResult::MemberMatched);
    assert_eq!(resp.error_code(), None);
}

#[test]
fn ac8_below_min_takes_precedence_and_alerts_once() {
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
    let mut svc = service(store, 1_000);

    let resp = svc.sign_in_ok(signin_req("+15550000001", ios_below_min()));

    assert_eq!(resp.result, SignInResult::BelowMinVersion);
    assert_eq!(resp.error_code(), Some("AUTH_BELOW_MIN_VERSION"));
    assert_eq!(svc.alerts.count_kind(AlertKind::BelowMinVersion), 1);
}

#[test]
fn signin_no_existence_leak_same_shape_matched_vs_unmatched() {
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
    let mut svc = service(store, 1_000);

    let matched = svc.sign_in_ok(signin_req("+15550000001", ios_current()));
    let unmatched = svc.sign_in_ok(signin_req("+15559999999", ios_current()));

    // Identical version + manifest pointer; the responses differ ONLY in the result discriminant
    // (the legitimate signal the helper needs) — no extra field branches on existence.
    assert_eq!(matched.version, unmatched.version);
    assert_eq!(matched.manifest_pointer, unmatched.manifest_pointer);
    assert_eq!(matched.result, SignInResult::MemberMatched);
    assert_eq!(unmatched.result, SignInResult::PhoneNotOnFile);
    // A miss reveals nothing beyond "not on file"; no alert and no member info.
    assert!(svc.alerts.alerts.is_empty());
}

#[test]
fn signin_below_min_collapses_match_and_is_identical() {
    let mut store = MemStore::new();
    store.add_member(member_id(1), "+15550000001", vec![Role::Rider]);
    let mut svc = service(store, 1_000);

    let matched = svc.sign_in_ok(signin_req("+15550000001", ios_below_min()));
    let unmatched = svc.sign_in_ok(signin_req("+15559999999", ios_below_min()));

    // Below-min collapses both to the same degradation response — match is not revealed.
    assert_eq!(matched.result, SignInResult::BelowMinVersion);
    assert_eq!(matched, unmatched);
    // Only the real member's below-min produces an admin alert (no member to help on a miss); the
    // alert goes to the admin via Queues, never the client, so the responses stay identical.
    assert_eq!(svc.alerts.count_kind(AlertKind::BelowMinVersion), 1);
}
