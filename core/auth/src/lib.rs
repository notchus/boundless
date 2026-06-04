//! `boundless-auth` — device-side authentication & onboarding logic (ADR-0016, spec 001).
//!
//! The single source of truth (P4) for how an already-issued member's device transitions
//! through first-launch and how the client and server negotiate version compatibility.
//! Generated to Swift/Kotlin (wired in **T10**) so every platform decides identically;
//! clients render states, they never re-implement the rules.
//!
//! ## What lands in T04 (this slice)
//! - [`clock`] — an **injected** [`Clock`] over [`UnixSeconds`] (UTC); the core never calls
//!   `SystemTime::now` (forbidden-patterns). Production supplies server time; tests use
//!   [`FixedClock`].
//! - [`version`] — the O4 below-minimum gate ([`evaluate_version`]) and the O1 N-2 support
//!   window ([`minimum_supported`]).
//! - [`code`] — the AC17 Onboarding Code lifecycle decision ([`evaluate_onboarding_code`]:
//!   single-use / TTL / rate-limit / regenerate-invalidates-prior) and the AC19 driver-only
//!   Recovery Code path ([`evaluate_recovery_code`], [`recovery_available_for`]). The
//!   constant-time secret match composes `boundless_crypto`, keeping the decision order
//!   single-sourced (no timing oracle).
//! - [`state`] — the [`OnboardingState`] machine ([`OnboardingState::on_event`]) plus the
//!   AC8/AC15 routing decisions ([`reauth_state_for`], the below-min transition).
//!
//! ## Deliberately **not** in T04 (see `DEFERRED.md` → T04)
//! Server-time enforcement, the rate-limit window bookkeeping, Turnstile, and the Queue
//! admin alerts are **T07**. Indefinite sessions, silent refresh-token rotation with
//! replay/lineage detection, and device-token binding are **T05**. The UniFFI export of this
//! surface is **T10**.

mod clock;
mod code;
mod state;
mod version;

pub use clock::{Clock, FixedClock, UnixSeconds};
pub use code::{
    evaluate_onboarding_code, evaluate_recovery_code, recovery_available_for,
    OnboardingCodeChallenge, OnboardingCodeVerdict, RecoveryChallenge, RecoveryCodeVerdict,
};
pub use state::{
    launch, reauth_state_for, should_flag_notifications_off, BindResult, LaunchDecision,
    OnboardingEvent, OnboardingState, SignInResult, SESSION_INVALIDATED_CODE,
};
pub use version::{evaluate_version, minimum_supported, VersionRequirement, VersionVerdict};
