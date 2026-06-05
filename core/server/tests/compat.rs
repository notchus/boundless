//! End-to-end N-2 compatibility (spec 001 **T08**, AC9 — the engine half): the real
//! `AuthService::sign_in` orchestration accepts a client on the current minor and the two previous
//! supported minors, and degrades only a client below the floor (O1/O4/O8). The **fixture-replay**
//! half of AC9 (`ac9_auth_endpoints_nminus2`, replaying the frozen `fixtures/compat/**` request
//! shapes over the version policy) lives in `server/tests/compat/` where the task mandates it; this
//! proves the *endpoint itself* never branches observably on an in-window version.

mod common;

use boundless_auth::SignInResult;
use boundless_domain::{Platform, Role};
use boundless_server_core::{normalize_phone, SignInRequest};

use common::*;

const PHONE: &str = "+15550000001";

fn signin(version: (u32, u32, u32)) -> SignInRequest {
    SignInRequest {
        phone: normalize_phone(PHONE).expect("valid E.164"),
        reported: client_version(Platform::Ios, version.0, version.1, version.2),
    }
}

#[test]
fn compat_signin_accepts_n_minus_2_window_and_degrades_below_floor() {
    // The default requirement is client_min_version = 1.0.0 (= minimum_supported(1.2.0, 2)),
    // recommended 1.2.0 — the N-2 window for a current 1.2 server.
    let mut store = MemStore::new();
    store.add_member(member_id(1), PHONE, vec![Role::Rider]);
    let mut svc = service(store, 1_000);

    // current (1.2.0), n-1 (matches fixtures/compat/n_minus_1 = 1.1.3), n-2 (n_minus_2 = 1.0.7),
    // and exactly the floor (1.0.0) are all accepted — the server returns the real lookup result.
    for v in [(1, 2, 0), (1, 1, 3), (1, 0, 7), (1, 0, 0)] {
        assert_eq!(
            svc.sign_in_ok(signin(v)).result,
            SignInResult::MemberMatched,
            "client {v:?} is within the N-2 window and must be accepted"
        );
    }

    // Below the floor (one patch under 1.0.0 is impossible, so use 0.9.x): calm degradation only —
    // never a match/miss answer the rider could act on (O4/O8).
    assert_eq!(
        svc.sign_in_ok(signin((0, 9, 9))).result,
        SignInResult::BelowMinVersion
    );
}
