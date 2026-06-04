//! Onboarding / Recovery code lifecycle decisions (ADR-0016 D1/D3; spec AC17/AC19).
//!
//! This is the **core leg** of code validation: a single, pure decision function per code
//! kind that takes the persisted challenge state plus the injected clock and returns a
//! verdict. The server (**T07**) owns the *enforcement* around it — server-time, the
//! Postgres rows, the rate-limit window bookkeeping, Turnstile, and the lock alert — but the
//! *decision order* lives here so it is single-sourced (P4) and cannot drift.
//!
//! ## Why the order is fixed here (no timing oracle)
//!
//! The non-secret lifecycle gates (consumed / superseded / rate-limited / expired) are
//! checked **first**; only if all pass does the function reach the **one** secret-dependent
//! branch, which uses `boundless_crypto`'s constant-time matcher. None of the early gates
//! depend on the *presented* secret, so they leak nothing about it; the secret comparison is
//! constant-time (I3 / R2). Pulling that order into one core function stops the server from
//! accidentally comparing the secret before the gates and opening an oracle.

use boundless_crypto::{onboarding_code_matches, recovery_code_matches, CodeHash, HmacKey};
use boundless_domain::{OnboardingCode, RecoveryCode, Role};
use serde::{Deserialize, Serialize};

use crate::clock::Clock;

/// The persisted state of an outstanding Onboarding Code, as the server holds it (ADR-0016
/// D1). Holds the at-rest **hash** only — never the code itself (the plaintext is touched
/// only at the `boundless_crypto` boundary).
#[derive(Clone)]
pub struct OnboardingCodeChallenge {
    /// HMAC-SHA256 of the issued code, keyed per-instance (`boundless_crypto`).
    pub code_hash: CodeHash,
    /// Server-side TTL boundary; at or after this instant the code is [`Expired`]
    /// (default 72h, plan §10-D). Compared against the **injected** clock, never the device.
    ///
    /// [`Expired`]: OnboardingCodeVerdict::Expired
    pub expires_at: crate::clock::UnixSeconds,
    /// Maximum bind attempts within the rate-limit window (default 5, plan §10-D).
    pub max_attempts: u32,
    /// Attempts already made within the current rate-limit window. The **window itself**
    /// (default 15 min) is server bookkeeping (T07); this is the count the server supplies.
    pub recent_attempts: u32,
    /// Already used (single-use): a successful bind sets this. Never accepts again.
    pub consumed: bool,
    /// Superseded by a regenerated code — regenerate-invalidates-prior (AC17). Never accepts.
    pub superseded: bool,
}

/// The outcome of evaluating a presented Onboarding Code against its challenge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnboardingCodeVerdict {
    /// Live and correct — bind the device (the server then marks the challenge consumed).
    Accepted,
    /// Live but the presented code did not match.
    Invalid,
    /// Past its server-side TTL.
    Expired,
    /// Already used, or superseded by a regenerated code.
    Consumed,
    /// Too many attempts in the window — locked (the server alerts the admin).
    RateLimited,
}

impl OnboardingCodeVerdict {
    /// The stable error code for this verdict, or `None` when accepted
    /// (`docs/error-codes.md`, P12).
    pub const fn error_code(self) -> Option<&'static str> {
        match self {
            Self::Accepted => None,
            Self::Invalid => Some("AUTH_ONBOARDING_CODE_INVALID"),
            Self::Expired => Some("AUTH_ONBOARDING_CODE_EXPIRED"),
            Self::Consumed => Some("AUTH_ONBOARDING_CODE_CONSUMED"),
            Self::RateLimited => Some("AUTH_ONBOARDING_CODE_RATE_LIMITED"),
        }
    }

    /// Whether the device may proceed to the `Permissions` step.
    pub const fn is_accepted(self) -> bool {
        matches!(self, Self::Accepted)
    }
}

/// Evaluate a presented Onboarding Code (AC17). See the module docs for why the gate order
/// is fixed. `clock` is the **injected** time source (server time in production); the device
/// clock never participates, so a wrong device clock cannot grant or deny a bind.
pub fn evaluate_onboarding_code(
    challenge: &OnboardingCodeChallenge,
    presented: &OnboardingCode,
    key: &HmacKey,
    clock: &impl Clock,
) -> OnboardingCodeVerdict {
    if challenge.consumed || challenge.superseded {
        return OnboardingCodeVerdict::Consumed;
    }
    if challenge.recent_attempts >= challenge.max_attempts {
        return OnboardingCodeVerdict::RateLimited;
    }
    if clock.now() >= challenge.expires_at {
        return OnboardingCodeVerdict::Expired;
    }
    if onboarding_code_matches(key, presented, &challenge.code_hash) {
        OnboardingCodeVerdict::Accepted
    } else {
        OnboardingCodeVerdict::Invalid
    }
}

/// The persisted state of a Driver's outstanding Recovery Code (ADR-0016 D3). Driver-held,
/// **no TTL** (rotated on use, not time-expired), so there is no `expires_at` and the device
/// clock is irrelevant here.
#[derive(Clone)]
pub struct RecoveryChallenge {
    /// HMAC-SHA256 of the issued Recovery Code (distinct domain tag from Onboarding Codes,
    /// so an Onboarding Code can never verify as a Recovery Code — `boundless_crypto`).
    pub code_hash: CodeHash,
    /// Already used (single-use): consumed on a successful re-bind.
    pub consumed: bool,
    /// Superseded by a freshly-issued Recovery Code (rotated on use).
    pub superseded: bool,
}

/// The outcome of evaluating a presented Recovery Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryCodeVerdict {
    /// Live and correct — re-bind the new device (old token invalidated, fresh code issued).
    Accepted,
    /// Did not match, already consumed, or superseded.
    Invalid,
    /// Self-serve recovery is **not available for this member's role** — only Drivers
    /// self-serve; Riders (and Admins) recover via an Admin re-issue (ADR-0016 D3, P10).
    /// Returned before any code comparison, so a non-Driver attempt never probes the secret.
    NotAvailable,
}

impl RecoveryCodeVerdict {
    /// The stable error code for this verdict, or `None` when accepted (P12).
    pub const fn error_code(self) -> Option<&'static str> {
        match self {
            Self::Accepted => None,
            Self::Invalid => Some("AUTH_RECOVERY_CODE_INVALID"),
            Self::NotAvailable => Some("AUTH_RECOVERY_NOT_AVAILABLE"),
        }
    }

    /// Whether the new device may be re-bound.
    pub const fn is_accepted(self) -> bool {
        matches!(self, Self::Accepted)
    }
}

/// Evaluate a presented Driver Recovery Code (AC19).
///
/// The **driver-only role gate is enforced here, not delegated to the caller** — so the rule
/// is single-sourced in the core (P4) and cannot drift: a non-Driver returns `NotAvailable`
/// before any secret comparison (the same reason the onboarding gate order lives in one
/// function — see the module docs). Otherwise single-use (`consumed`/`superseded`) and the
/// constant-time match gate it. There is **no TTL** — recovery codes are rotated on use, not
/// time-expired (ADR-0016 D3), so no clock participates.
pub fn evaluate_recovery_code(
    challenge: &RecoveryChallenge,
    presented: &RecoveryCode,
    key: &HmacKey,
    role: Role,
) -> RecoveryCodeVerdict {
    if !recovery_available_for(role) {
        return RecoveryCodeVerdict::NotAvailable;
    }
    if challenge.consumed || challenge.superseded {
        return RecoveryCodeVerdict::Invalid;
    }
    if recovery_code_matches(key, presented, &challenge.code_hash) {
        RecoveryCodeVerdict::Accepted
    } else {
        RecoveryCodeVerdict::Invalid
    }
}

/// Whether self-serve recovery (Recovery Code re-bind) is available for `role` (ADR-0016 D3,
/// AC19). **Only Drivers** self-serve; Riders recover exclusively via an Admin re-issue
/// (P10 — no rider homework). [`evaluate_recovery_code`] enforces this internally; this is
/// exposed so the server can also short-circuit (returning `AUTH_RECOVERY_NOT_AVAILABLE`)
/// before it even loads a challenge.
pub const fn recovery_available_for(role: Role) -> bool {
    matches!(role, Role::Driver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{FixedClock, UnixSeconds};
    use boundless_crypto::{onboarding_code_hash, HMAC_KEY_LEN};

    fn key() -> HmacKey {
        HmacKey::from_bytes([7u8; HMAC_KEY_LEN])
    }

    #[test]
    fn recovery_only_for_drivers() {
        assert!(recovery_available_for(Role::Driver));
        assert!(!recovery_available_for(Role::Rider));
        assert!(!recovery_available_for(Role::Admin));
    }

    #[test]
    fn live_correct_code_is_accepted() {
        let k = key();
        let code = OnboardingCode::new("hunter2-correct-horse");
        let challenge = OnboardingCodeChallenge {
            code_hash: onboarding_code_hash(&k, &code),
            expires_at: UnixSeconds::new(2_000),
            max_attempts: 5,
            recent_attempts: 0,
            consumed: false,
            superseded: false,
        };
        let v = evaluate_onboarding_code(&challenge, &code, &k, &FixedClock::at_secs(1_000));
        assert_eq!(v, OnboardingCodeVerdict::Accepted);
        assert_eq!(v.error_code(), None);
    }
}
