//! P12: every error/operational code the `core::server` engine can surface must be registered in
//! `docs/error-codes.md` (mirrors `core/auth`'s `auth_verdict_error_codes_match_registry`).

mod common;

use boundless_auth::{OnboardingCodeVerdict, RecoveryCodeVerdict, RefreshVerdict};
use boundless_domain::AppVersion;
use boundless_server_core::{AdminAlert, GroupKeyMissing};
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

/// Spec 008 (admin member-management / issuance) registers its five issuance error codes at T01
/// (P12: the code is documented before the emitting type ships). The emitting enum
/// (`MemberError::error_code`) lands at T05; this forward-looking check guarantees the registry
/// already carries every code that slice will surface, so T05 cannot introduce an uncoded variant.
#[test]
fn admin_member_issuance_codes_registered() {
    let reg = registry();

    for code in [
        "ADMIN_MEMBER_PHONE_INVALID",
        "ADMIN_MEMBER_ADDRESS_INVALID",
        "ADMIN_MEMBER_DUPLICATE_PHONE",
        "ADMIN_MEMBER_EDIT_STALE",
        "ADMIN_GROUP_KEY_MISSING",
    ] {
        let cell = format!("`{code}`");
        assert!(
            reg.contains(&cell),
            "spec-008 issuance code {code} is not registered as a code cell in docs/error-codes.md (P12)"
        );
    }
}

/// The first spec-008 *emitting type* (T04's Group-bootstrap fail-closed gate) must surface a
/// **registered** code: ties `GroupKeyMissing::error_code()` to the literal and to the registry, so a
/// future typo in either the constant or the doc fails CI (the stronger form of the forward-looking
/// `admin_member_issuance_codes_registered` check, now that the type exists).
#[test]
fn group_key_missing_error_code_registered() {
    let code = GroupKeyMissing.error_code();
    assert_eq!(code, "ADMIN_GROUP_KEY_MISSING");
    assert!(
        registry().contains(&format!("`{code}`")),
        "GroupKeyMissing's code {code} is not registered as a code cell in docs/error-codes.md (P12)"
    );
}
