//! Onboarding / Recovery code lifecycle tests (spec AC17 / AC19; ADR-0016 D1/D3).
//!
//! The centerpiece is the property `prop_onboarding_code_single_use_ttl_ratelimit`, which
//! pins the whole decision: **Accepted ⟺ (live ∧ correct)**, with the documented gate
//! precedence for every rejection. Deterministic boundary units pin the exact edges
//! (P9 — reproducible without relying on the RNG); any failing random case additionally
//! persists its seed to the committed `proptest-regressions/` directory.

use boundless_auth::{
    evaluate_onboarding_code, evaluate_recovery_code, recovery_available_for, FixedClock,
    OnboardingCodeChallenge, OnboardingCodeVerdict, RecoveryChallenge, RecoveryCodeVerdict,
    UnixSeconds,
};
use boundless_crypto::{onboarding_code_hash, recovery_code_hash, HmacKey, HMAC_KEY_LEN};
use boundless_domain::{OnboardingCode, RecoveryCode, Role};
use proptest::prelude::*;

fn key() -> HmacKey {
    HmacKey::from_bytes([0x5Au8; HMAC_KEY_LEN])
}

/// The expected verdict, written out independently of the implementation so the test pins
/// the gate precedence rather than mirroring the code.
fn expected_onboarding_verdict(
    consumed: bool,
    superseded: bool,
    recent_attempts: u32,
    max_attempts: u32,
    now: i64,
    expires_at: i64,
    presented_correct: bool,
) -> OnboardingCodeVerdict {
    if consumed || superseded {
        OnboardingCodeVerdict::Consumed
    } else if recent_attempts >= max_attempts {
        OnboardingCodeVerdict::RateLimited
    } else if now >= expires_at {
        OnboardingCodeVerdict::Expired
    } else if presented_correct {
        OnboardingCodeVerdict::Accepted
    } else {
        OnboardingCodeVerdict::Invalid
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1024, .. ProptestConfig::default() })]

    /// AC17: single-use, TTL, and rate-limit, all at once. For any lifecycle state +
    /// injected time + correct-or-wrong code, the verdict equals the independently-computed
    /// expectation — so `Accepted` happens **iff** the code is live (not consumed, not
    /// superseded, attempts under the limit, before expiry) **and** matches.
    #[test]
    fn prop_onboarding_code_single_use_ttl_ratelimit(
        consumed in any::<bool>(),
        superseded in any::<bool>(),
        recent_attempts in 0u32..8,
        max_attempts in 0u32..8,
        now in 0i64..1_000_000,
        expires_at in 0i64..1_000_000,
        presented_correct in any::<bool>(),
    ) {
        let k = key();
        let correct = OnboardingCode::new("correct-onboarding-code");
        let wrong = OnboardingCode::new("a-different-code-entirely");
        let presented = if presented_correct { correct.clone() } else { wrong };

        let challenge = OnboardingCodeChallenge {
            code_hash: onboarding_code_hash(&k, &correct),
            expires_at: UnixSeconds::new(expires_at),
            max_attempts,
            recent_attempts,
            consumed,
            superseded,
        };

        let verdict = evaluate_onboarding_code(&challenge, &presented, &k, &FixedClock::at_secs(now));
        let expected = expected_onboarding_verdict(
            consumed, superseded, recent_attempts, max_attempts, now, expires_at, presented_correct,
        );
        prop_assert_eq!(verdict, expected);

        // The core invariant stated directly.
        let live = !consumed && !superseded && recent_attempts < max_attempts && now < expires_at;
        prop_assert_eq!(verdict.is_accepted(), live && presented_correct);
    }
}

#[test]
fn ac17_regenerate_invalidates_prior() {
    // When the Admin regenerates, the prior code's challenge is marked `superseded`. Even the
    // correct prior code, otherwise live, no longer binds (regenerate-invalidates-prior).
    let k = key();
    let prior = OnboardingCode::new("prior-code-abc123");
    let challenge = OnboardingCodeChallenge {
        code_hash: onboarding_code_hash(&k, &prior),
        expires_at: UnixSeconds::new(10_000),
        max_attempts: 5,
        recent_attempts: 0,
        consumed: false,
        superseded: true,
    };
    let verdict = evaluate_onboarding_code(&challenge, &prior, &k, &FixedClock::at_secs(1));
    assert_eq!(verdict, OnboardingCodeVerdict::Consumed);
    assert_eq!(verdict.error_code(), Some("AUTH_ONBOARDING_CODE_CONSUMED"));
}

#[test]
fn single_use_consumed_code_never_accepts_again() {
    let k = key();
    let code = OnboardingCode::new("one-shot-code");
    let challenge = OnboardingCodeChallenge {
        code_hash: onboarding_code_hash(&k, &code),
        expires_at: UnixSeconds::new(10_000),
        max_attempts: 5,
        recent_attempts: 0,
        consumed: true,
        superseded: false,
    };
    assert_eq!(
        evaluate_onboarding_code(&challenge, &code, &k, &FixedClock::at_secs(1)),
        OnboardingCodeVerdict::Consumed
    );
}

#[test]
fn boundary_expiry_is_inclusive_at_now() {
    let k = key();
    let code = OnboardingCode::new("ttl-boundary-code");
    let mk = |exp: i64| OnboardingCodeChallenge {
        code_hash: onboarding_code_hash(&k, &code),
        expires_at: UnixSeconds::new(exp),
        max_attempts: 5,
        recent_attempts: 0,
        consumed: false,
        superseded: false,
    };
    // now == expires_at → Expired (the boundary is `now >= expires_at`).
    assert_eq!(
        evaluate_onboarding_code(&mk(1_000), &code, &k, &FixedClock::at_secs(1_000)),
        OnboardingCodeVerdict::Expired
    );
    // one second before expiry → still live → Accepted.
    assert_eq!(
        evaluate_onboarding_code(&mk(1_000), &code, &k, &FixedClock::at_secs(999)),
        OnboardingCodeVerdict::Accepted
    );
}

#[test]
fn boundary_rate_limit_at_max_attempts() {
    let k = key();
    let code = OnboardingCode::new("rate-limit-boundary-code");
    let mk = |attempts: u32| OnboardingCodeChallenge {
        code_hash: onboarding_code_hash(&k, &code),
        expires_at: UnixSeconds::new(10_000),
        max_attempts: 5,
        recent_attempts: attempts,
        consumed: false,
        superseded: false,
    };
    // one below the limit → still allowed.
    assert_eq!(
        evaluate_onboarding_code(&mk(4), &code, &k, &FixedClock::at_secs(1)),
        OnboardingCodeVerdict::Accepted
    );
    // at the limit → locked, even with the correct code.
    assert_eq!(
        evaluate_onboarding_code(&mk(5), &code, &k, &FixedClock::at_secs(1)),
        OnboardingCodeVerdict::RateLimited
    );
}

#[test]
fn boundary_max_attempts_zero_is_locked_from_issuance() {
    // A degenerate `max_attempts: 0` locks the code before any attempt (`0 >= 0`). This is
    // never a production value (plan §10-D fixes 5), but the verdict is pinned so a T07
    // mis-wiring surfaces as this documented behavior rather than a silent surprise.
    let k = key();
    let code = OnboardingCode::new("zero-max-attempts-code");
    let challenge = OnboardingCodeChallenge {
        code_hash: onboarding_code_hash(&k, &code),
        expires_at: UnixSeconds::new(10_000),
        max_attempts: 0,
        recent_attempts: 0,
        consumed: false,
        superseded: false,
    };
    assert_eq!(
        evaluate_onboarding_code(&challenge, &code, &k, &FixedClock::at_secs(1)),
        OnboardingCodeVerdict::RateLimited
    );
}

#[test]
fn code_validation_uses_injected_clock_not_device_time() {
    // The verdict depends solely on the injected clock; `core::auth` never reads a device
    // clock, so a wrong device clock can neither grant nor deny a bind (spec "User's clock is
    // wrong"). We prove it by moving only the injected clock across the TTL boundary.
    let k = key();
    let code = OnboardingCode::new("server-time-decides");
    let challenge = OnboardingCodeChallenge {
        code_hash: onboarding_code_hash(&k, &code),
        expires_at: UnixSeconds::new(1_000),
        max_attempts: 5,
        recent_attempts: 0,
        consumed: false,
        superseded: false,
    };
    assert_eq!(
        evaluate_onboarding_code(&challenge, &code, &k, &FixedClock::at_secs(999)),
        OnboardingCodeVerdict::Accepted
    );
    assert_eq!(
        evaluate_onboarding_code(&challenge, &code, &k, &FixedClock::at_secs(1_000)),
        OnboardingCodeVerdict::Expired
    );
    // A far-future injected time (e.g. a misconfigured server) still only reads what it is
    // given — there is no hidden device-time path that could disagree.
    assert_eq!(
        evaluate_onboarding_code(&challenge, &code, &k, &FixedClock::at_secs(5_000)),
        OnboardingCodeVerdict::Expired
    );
}

#[test]
fn ac19_driver_recovery_code_rebind() {
    assert!(recovery_available_for(Role::Driver));
    let k = key();
    let code = RecoveryCode::new("driver-held-recovery-secret");

    // Live + correct → Accepted (the server then invalidates the old token and rotates).
    let live = RecoveryChallenge {
        code_hash: recovery_code_hash(&k, &code),
        consumed: false,
        superseded: false,
    };
    assert_eq!(
        evaluate_recovery_code(&live, &code, &k, Role::Driver),
        RecoveryCodeVerdict::Accepted
    );

    // Reused (single-use) → Invalid.
    let used = RecoveryChallenge {
        code_hash: recovery_code_hash(&k, &code),
        consumed: true,
        superseded: false,
    };
    assert_eq!(
        evaluate_recovery_code(&used, &code, &k, Role::Driver),
        RecoveryCodeVerdict::Invalid
    );

    // Superseded by a rotated code → Invalid.
    let rotated = RecoveryChallenge {
        code_hash: recovery_code_hash(&k, &code),
        consumed: false,
        superseded: true,
    };
    assert_eq!(
        evaluate_recovery_code(&rotated, &code, &k, Role::Driver),
        RecoveryCodeVerdict::Invalid
    );

    // Wrong code → Invalid.
    let wrong = RecoveryCode::new("not-the-recovery-code");
    assert_eq!(
        evaluate_recovery_code(&live, &wrong, &k, Role::Driver),
        RecoveryCodeVerdict::Invalid
    );
    assert_eq!(
        RecoveryCodeVerdict::Invalid.error_code(),
        Some("AUTH_RECOVERY_CODE_INVALID")
    );
}

#[test]
fn ac19_rider_has_no_self_serve_recovery() {
    assert!(!recovery_available_for(Role::Rider));
    assert!(!recovery_available_for(Role::Admin));

    // The role gate is enforced *inside* evaluate_recovery_code (P4 — not a caller contract):
    // a non-Driver gets NotAvailable even with an otherwise-live, correct code.
    let k = key();
    let code = RecoveryCode::new("would-be-valid-code");
    let live = RecoveryChallenge {
        code_hash: recovery_code_hash(&k, &code),
        consumed: false,
        superseded: false,
    };
    for role in [Role::Rider, Role::Admin] {
        let verdict = evaluate_recovery_code(&live, &code, &k, role);
        assert_eq!(verdict, RecoveryCodeVerdict::NotAvailable);
        assert_eq!(verdict.error_code(), Some("AUTH_RECOVERY_NOT_AVAILABLE"));
    }
}

/// AC19 recovery as a property, symmetric with the onboarding property: a recovery code is
/// `Accepted` **iff** the role is Driver, the code is live (not consumed/superseded), and it
/// matches. No time dimension participates (recovery has no TTL by design).
fn expected_recovery_verdict(
    is_driver: bool,
    consumed: bool,
    superseded: bool,
    presented_correct: bool,
) -> RecoveryCodeVerdict {
    if !is_driver {
        RecoveryCodeVerdict::NotAvailable
    } else if consumed || superseded {
        RecoveryCodeVerdict::Invalid
    } else if presented_correct {
        RecoveryCodeVerdict::Accepted
    } else {
        RecoveryCodeVerdict::Invalid
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, .. ProptestConfig::default() })]

    #[test]
    fn prop_recovery_code_single_use(
        is_driver in any::<bool>(),
        consumed in any::<bool>(),
        superseded in any::<bool>(),
        presented_correct in any::<bool>(),
    ) {
        let k = key();
        let correct = RecoveryCode::new("correct-recovery-code");
        let wrong = RecoveryCode::new("a-different-recovery-code");
        let presented = if presented_correct { correct.clone() } else { wrong };
        let role = if is_driver { Role::Driver } else { Role::Rider };

        let challenge = RecoveryChallenge {
            code_hash: recovery_code_hash(&k, &correct),
            consumed,
            superseded,
        };
        let verdict = evaluate_recovery_code(&challenge, &presented, &k, role);
        prop_assert_eq!(
            verdict,
            expected_recovery_verdict(is_driver, consumed, superseded, presented_correct)
        );
        let live = is_driver && !consumed && !superseded;
        prop_assert_eq!(verdict.is_accepted(), live && presented_correct);
    }
}
