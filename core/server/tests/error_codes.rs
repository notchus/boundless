//! P12: every error/operational code the `core::server` engine can surface must be registered in
//! `docs/error-codes.md` (mirrors `core/auth`'s `auth_verdict_error_codes_match_registry`).

mod common;

use boundless_auth::{OnboardingCodeVerdict, RecoveryCodeVerdict, RefreshVerdict};
use boundless_domain::AppVersion;
use boundless_server_core::AdminAlert;
use common::member_id;
use std::path::Path;

fn registry() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/error-codes.md");
    std::fs::read_to_string(p).expect("read docs/error-codes.md")
}

#[test]
fn server_verdict_error_codes_match_registry() {
    let reg = registry();

    let mut codes: Vec<&str> = Vec::new();

    // Sign-in (the literals `SignInResponse::error_code` surfaces).
    codes.push("AUTH_PHONE_NOT_ON_FILE");
    codes.push("AUTH_BELOW_MIN_VERSION");

    // Device-bind (Onboarding Code verdicts).
    for v in [
        OnboardingCodeVerdict::Invalid,
        OnboardingCodeVerdict::Expired,
        OnboardingCodeVerdict::Consumed,
        OnboardingCodeVerdict::RateLimited,
    ] {
        codes.push(v.error_code().unwrap());
    }

    // Recovery re-bind.
    codes.push(RecoveryCodeVerdict::Invalid.error_code().unwrap());
    codes.push(RecoveryCodeVerdict::NotAvailable.error_code().unwrap());

    // Refresh (server-side operational verdicts).
    codes.push(
        RefreshVerdict::ReplayDetectedKillFamily
            .error_code()
            .unwrap(),
    );
    codes.push(RefreshVerdict::Rejected.error_code().unwrap());

    // Admin alerts / flags — including the AC14 operational flag introduced by this slice.
    for a in [
        AdminAlert::BelowMinVersion {
            member: member_id(1),
            reported_version: AppVersion::new(0, 9, 0),
        },
        AdminAlert::SessionInvalidated {
            member: member_id(1),
        },
        AdminAlert::OnboardingCodeLocked {
            member: member_id(1),
        },
        AdminAlert::NotificationsNotEnabled {
            member: member_id(1),
        },
    ] {
        codes.push(a.error_code());
    }

    for code in codes {
        // Match the whole backtick-wrapped code cell (the registry formats every code that way),
        // not a bare substring — so one code can't satisfy the check by being another's prefix.
        let cell = format!("`{code}`");
        assert!(
            reg.contains(&cell),
            "error code {code} is not registered as a code cell in docs/error-codes.md (P12)"
        );
    }
}
