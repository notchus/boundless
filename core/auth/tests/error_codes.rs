//! P12: every error code a `core::auth` verdict/decision can surface must be registered in
//! `docs/error-codes.md` (mirrors `core/crypto`'s `manifest_error_codes_match_registry`).

use boundless_auth::{
    OnboardingCodeVerdict, RecoveryCodeVerdict, VersionVerdict, SESSION_INVALIDATED_CODE,
};
use std::path::Path;

fn registry() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/error-codes.md");
    std::fs::read_to_string(p).expect("read docs/error-codes.md")
}

#[test]
fn auth_verdict_error_codes_match_registry() {
    let reg = registry();

    let mut codes: Vec<&str> = Vec::new();
    codes.push(VersionVerdict::BelowMinimum.error_code().unwrap());
    for v in [
        OnboardingCodeVerdict::Invalid,
        OnboardingCodeVerdict::Expired,
        OnboardingCodeVerdict::Consumed,
        OnboardingCodeVerdict::RateLimited,
    ] {
        codes.push(v.error_code().unwrap());
    }
    codes.push(RecoveryCodeVerdict::Invalid.error_code().unwrap());
    codes.push(RecoveryCodeVerdict::NotAvailable.error_code().unwrap());
    codes.push(SESSION_INVALIDATED_CODE);

    for code in codes {
        // Match the code as a whole backtick-wrapped table cell (`docs/error-codes.md` formats
        // every code that way), not a bare substring — so one code can't spuriously satisfy
        // the check by being a prefix of another.
        let cell = format!("`{code}`");
        assert!(
            reg.contains(&cell),
            "error code {code} is not registered as a code cell in docs/error-codes.md (P12)"
        );
    }

    // The success verdicts carry no code.
    assert_eq!(OnboardingCodeVerdict::Accepted.error_code(), None);
    assert_eq!(RecoveryCodeVerdict::Accepted.error_code(), None);
    assert_eq!(VersionVerdict::Supported.error_code(), None);
    assert_eq!(VersionVerdict::SupportedButOutdated.error_code(), None);
}
