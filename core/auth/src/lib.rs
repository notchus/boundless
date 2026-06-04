//! `boundless-auth` — device-side authentication & onboarding logic (ADR-0016, spec 001).
//!
//! The single source of truth (P4) for how an already-issued member's device transitions
//! through first-launch and how the client and server negotiate version compatibility.
//! Generated to Swift/Kotlin (wired in **T10**) so every platform decides identically;
//! clients render states, they never re-implement the rules.
//!
//! ## What lands in T04
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
//! ## What lands in T05 (this slice)
//! - [`session`] — the indefinite-session model (ADR-0016 D2, AC18): [`Session::is_live`] is
//!   time-independent; [`Session::needs_refresh`] is the silent-refresh trigger; and
//!   [`evaluate_refresh`] is the refresh-rotation policy with **replay detection** — a
//!   replayed credential revokes the whole family ([`RefreshVerdict::ReplayDetectedKillFamily`]).
//! - [`device`] — the I4 device-token binding tuple ([`DeviceBinding`]) and the exhaustive,
//!   admin-mediated invalidation triggers ([`invalidation_for`], [`reonboarding_invalidation`]:
//!   AC4) plus the §10-F secure-store contract ([`required_refresh_store`]).
//!
//! ## Deliberately **not** here (see `DEFERRED.md` → `core::auth` T04/T05)
//! Server-time enforcement, the rate-limit window bookkeeping, Turnstile, the Queue admin
//! alerts, the refresh credential's at-rest hashing + lineage DB lookup + classification, and
//! access-token signing are **T07**. The UniFFI export of this surface is **T10**; the
//! per-platform secure-store wiring is **T11–T15**.

mod clock;
mod code;
mod device;
mod session;
mod state;
mod version;

pub use clock::{Clock, FixedClock, UnixSeconds};
pub use code::{
    evaluate_onboarding_code, evaluate_recovery_code, recovery_available_for,
    OnboardingCodeChallenge, OnboardingCodeVerdict, RecoveryChallenge, RecoveryCodeVerdict,
};
pub use device::{
    invalidation_for, reonboarding_invalidation, required_refresh_store, DeviceBinding,
    InvalidationTrigger, SecureStoreClass, SessionInvalidation, TokenInvalidationScope,
    DEVICE_TOKEN_INVALIDATED_CODE,
};
pub use session::{
    evaluate_refresh, RefreshPresentation, RefreshVerdict, Session, SessionFamilyStatus,
};
pub use state::{
    launch, reauth_state_for, should_flag_notifications_off, BindResult, LaunchDecision,
    OnboardingEvent, OnboardingState, SignInResult, SESSION_INVALIDATED_CODE,
};
pub use version::{evaluate_version, minimum_supported, VersionRequirement, VersionVerdict};
