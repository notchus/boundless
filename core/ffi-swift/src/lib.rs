//! `boundless-ffi-swift` — UniFFI binding crate that produces the `BoundlessKit`
//! XCFramework for the Apple platforms (docs/architecture.md §1; spec 001 T10-shell).
//!
//! ## What this crate is (and is not)
//! It is the **only** place UniFFI touches Boundless. The wasm-safe core crates
//! (`boundless-domain`, `boundless-auth`) stay free of any `uniffi` dependency so they keep
//! compiling to `wasm32-unknown-unknown`; UniFFI cannot be added to them. Therefore this
//! crate **mirrors** the client-relevant core enums with `#[derive(uniffi::Enum)]` and bridges
//! them with **exhaustive `From` conversions** in both directions. The exhaustive `match` is the
//! parity guard: if a core variant is added or renamed, this crate fails to compile until the
//! conversion is updated — so the generated Swift can never silently drift from the core (P4).
//! This is a sanctioned codegen adapter, not a hand-rolled duplicate. See **ADR-0022**.
//!
//! That compile guard does NOT catch an **FFI-only** divergence (a `#[uniffi::export]` fn or mirror
//! variant added/renamed/re-signed here but not in `core/ffi-kotlin`, with the core unchanged). The
//! parity gate `tests/parity_with_kotlin.rs` does: it asserts the two crates' exported surfaces stay
//! byte-identical. Keep this `lib.rs` surface in lock-step with `core/ffi-kotlin/src/lib.rs`.
//!
//! ## Surface (client-relevant onboarding state machine only)
//! Clients **render states, they never decide them** — so this exposes the pure
//! `boundless_auth::state` graph: [`launch`], [`on_event`], [`is_terminal`],
//! [`allows_offline_overlay`], [`reauth_state_for`], [`should_flag_notifications_off`], plus the
//! enums they move between. The server-side decisions (code/refresh/version *evaluation*,
//! sessions, device-token invalidation) are **not** exported — clients receive their *outcomes*
//! over the HTTP/WS API (T07) and feed them in as [`OnboardingEvent`]s. No tainted/PII type and
//! no secret crosses this boundary (P2).
//!
//! Consumed by `apple/BoundlessKit/`. Generated bindings + the XCFramework are produced by
//! `scripts/build-boundlesskit.sh` and are NOT committed (reproducible from this source).

uniffi::setup_scaffolding!();

use boundless_auth as core_auth;
use boundless_domain as core_domain;

// ── Role ───────────────────────────────────────────────────────────────────────────────

/// Mirror of [`boundless_domain::Role`]. Wire form matches the core's `snake_case`.
#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A group member who needs transportation to the Gathering; in by default.
    Rider,
    /// A member with a car who has flipped their Seat Toggle on.
    Driver,
    /// A trusted member who manages membership; issued only by the Developer (I11).
    Admin,
}

impl From<core_domain::Role> for Role {
    fn from(r: core_domain::Role) -> Self {
        match r {
            core_domain::Role::Rider => Self::Rider,
            core_domain::Role::Driver => Self::Driver,
            core_domain::Role::Admin => Self::Admin,
        }
    }
}
impl From<Role> for core_domain::Role {
    fn from(r: Role) -> Self {
        match r {
            Role::Rider => Self::Rider,
            Role::Driver => Self::Driver,
            Role::Admin => Self::Admin,
        }
    }
}

// ── OnboardingState ──────────────────────────────────────────────────────────────────────

/// Mirror of [`boundless_auth::OnboardingState`] — a screen in the onboarding flow. `Offline`
/// and `ManifestFailReturning` are deliberately not states in the core (overlay / returning-
/// device outcome), so they are not here either.
#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnboardingState {
    /// Launched with no session and never onboarded.
    FreshInstall,
    /// Entering the member's phone number (sign-in).
    PhoneEntry,
    /// Phone matched; entering the Onboarding Code.
    DeviceBinding,
    /// Device bound, session issued; requesting notification permission.
    Permissions,
    /// Permission decision recorded; presenting the OS auto-update step (O3).
    AutoUpdateStep,
    /// Manifest applied (or fell back per ADR-0014 tiers); routes to the role's primary
    /// surface. Completion is silent (voice-and-tone).
    Complete,
    /// Phone-lookup miss; returns to `PhoneEntry`. Never reveals whether a number exists.
    PhoneNotOnFile,
    /// Onboarding Code invalid/expired/consumed/rate-limited; returns to `DeviceBinding`.
    BindingFailed,
    /// Client below `client_min_version` — the calm degradation screen, no "Update Now" (O4/O8).
    BelowMinVersion,
    /// A previously-valid Rider session was invalidated with no helper present — the calm help
    /// screen, never a sign-in form (AC15/P10). A Driver routes to `PhoneEntry` instead.
    NeedsReauthHelp,
}

impl From<core_auth::OnboardingState> for OnboardingState {
    fn from(s: core_auth::OnboardingState) -> Self {
        use core_auth::OnboardingState as C;
        match s {
            C::FreshInstall => Self::FreshInstall,
            C::PhoneEntry => Self::PhoneEntry,
            C::DeviceBinding => Self::DeviceBinding,
            C::Permissions => Self::Permissions,
            C::AutoUpdateStep => Self::AutoUpdateStep,
            C::Complete => Self::Complete,
            C::PhoneNotOnFile => Self::PhoneNotOnFile,
            C::BindingFailed => Self::BindingFailed,
            C::BelowMinVersion => Self::BelowMinVersion,
            C::NeedsReauthHelp => Self::NeedsReauthHelp,
        }
    }
}
impl From<OnboardingState> for core_auth::OnboardingState {
    fn from(s: OnboardingState) -> Self {
        match s {
            OnboardingState::FreshInstall => Self::FreshInstall,
            OnboardingState::PhoneEntry => Self::PhoneEntry,
            OnboardingState::DeviceBinding => Self::DeviceBinding,
            OnboardingState::Permissions => Self::Permissions,
            OnboardingState::AutoUpdateStep => Self::AutoUpdateStep,
            OnboardingState::Complete => Self::Complete,
            OnboardingState::PhoneNotOnFile => Self::PhoneNotOnFile,
            OnboardingState::BindingFailed => Self::BindingFailed,
            OnboardingState::BelowMinVersion => Self::BelowMinVersion,
            OnboardingState::NeedsReauthHelp => Self::NeedsReauthHelp,
        }
    }
}

// ── LaunchDecision ───────────────────────────────────────────────────────────────────────

/// Mirror of [`boundless_auth::LaunchDecision`].
#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchDecision {
    /// No live session → enter onboarding at [`OnboardingState::FreshInstall`].
    Onboard,
    /// A live session exists → resume straight to the role's primary surface.
    Resume,
}

impl From<core_auth::LaunchDecision> for LaunchDecision {
    fn from(d: core_auth::LaunchDecision) -> Self {
        match d {
            core_auth::LaunchDecision::Onboard => Self::Onboard,
            core_auth::LaunchDecision::Resume => Self::Resume,
        }
    }
}
impl From<LaunchDecision> for core_auth::LaunchDecision {
    fn from(d: LaunchDecision) -> Self {
        match d {
            LaunchDecision::Onboard => Self::Onboard,
            LaunchDecision::Resume => Self::Resume,
        }
    }
}

// ── SignInResult ─────────────────────────────────────────────────────────────────────────

/// Mirror of [`boundless_auth::SignInResult`] — the interpreted `/api/auth/signin` outcome the
/// client receives from the server and feeds into [`OnboardingEvent::SignIn`].
#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignInResult {
    /// Phone matched and version supported → proceed to device binding.
    MemberMatched,
    /// Phone-lookup miss.
    PhoneNotOnFile,
    /// The handshake reported the client below `client_min_version` (O4).
    BelowMinVersion,
}

impl From<core_auth::SignInResult> for SignInResult {
    fn from(r: core_auth::SignInResult) -> Self {
        match r {
            core_auth::SignInResult::MemberMatched => Self::MemberMatched,
            core_auth::SignInResult::PhoneNotOnFile => Self::PhoneNotOnFile,
            core_auth::SignInResult::BelowMinVersion => Self::BelowMinVersion,
        }
    }
}
impl From<SignInResult> for core_auth::SignInResult {
    fn from(r: SignInResult) -> Self {
        match r {
            SignInResult::MemberMatched => Self::MemberMatched,
            SignInResult::PhoneNotOnFile => Self::PhoneNotOnFile,
            SignInResult::BelowMinVersion => Self::BelowMinVersion,
        }
    }
}

// ── BindResult ───────────────────────────────────────────────────────────────────────────

/// Mirror of [`boundless_auth::BindResult`] — the interpreted `/api/auth/bind-device` outcome.
#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindResult {
    /// The Onboarding Code was accepted; the device is bound and a session issued.
    Bound,
    /// The Onboarding Code was rejected (invalid/expired/consumed/rate-limited).
    Failed,
}

impl From<core_auth::BindResult> for BindResult {
    fn from(r: core_auth::BindResult) -> Self {
        match r {
            core_auth::BindResult::Bound => Self::Bound,
            core_auth::BindResult::Failed => Self::Failed,
        }
    }
}
impl From<BindResult> for core_auth::BindResult {
    fn from(r: BindResult) -> Self {
        match r {
            BindResult::Bound => Self::Bound,
            BindResult::Failed => Self::Failed,
        }
    }
}

// ── OnboardingEvent ──────────────────────────────────────────────────────────────────────

/// Mirror of [`boundless_auth::OnboardingEvent`] — an input to the state machine. The core's
/// tuple variants are mirrored as named-field variants for a clearer Swift surface.
#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnboardingEvent {
    /// Begin sign-in from a fresh install.
    BeginSignIn,
    /// A `/api/auth/signin` response was interpreted.
    SignIn {
        /// The interpreted sign-in outcome.
        result: SignInResult,
    },
    /// Retry from the `PhoneNotOnFile` banner.
    RetryPhoneEntry,
    /// A `/api/auth/bind-device` response was interpreted.
    Bind {
        /// The interpreted bind outcome.
        result: BindResult,
    },
    /// Retry from the `BindingFailed` recovery screen.
    RetryBinding,
    /// The notification-permission decision was recorded (the flow advances either way, AC14).
    PermissionDecision {
        /// Whether notification permission was granted.
        granted: bool,
    },
    /// The OS auto-update step (O3) was confirmed complete.
    AutoUpdateConfirmed,
    /// A previously-valid session was invalidated mid-life; routes per [`reauth_state_for`].
    SessionInvalidated {
        /// The member's role, which decides Rider (help screen) vs Driver (re-auth) routing.
        role: Role,
    },
    /// A below-`client_min_version` handshake was detected (O4); reachable from any state.
    BelowMinVersionDetected,
}

impl From<OnboardingEvent> for core_auth::OnboardingEvent {
    fn from(e: OnboardingEvent) -> Self {
        match e {
            OnboardingEvent::BeginSignIn => Self::BeginSignIn,
            OnboardingEvent::SignIn { result } => Self::SignIn(result.into()),
            OnboardingEvent::RetryPhoneEntry => Self::RetryPhoneEntry,
            OnboardingEvent::Bind { result } => Self::Bind(result.into()),
            OnboardingEvent::RetryBinding => Self::RetryBinding,
            OnboardingEvent::PermissionDecision { granted } => Self::PermissionDecision { granted },
            OnboardingEvent::AutoUpdateConfirmed => Self::AutoUpdateConfirmed,
            OnboardingEvent::SessionInvalidated { role } => {
                Self::SessionInvalidated { role: role.into() }
            }
            OnboardingEvent::BelowMinVersionDetected => Self::BelowMinVersionDetected,
        }
    }
}
impl From<core_auth::OnboardingEvent> for OnboardingEvent {
    fn from(e: core_auth::OnboardingEvent) -> Self {
        use core_auth::OnboardingEvent as C;
        match e {
            C::BeginSignIn => Self::BeginSignIn,
            C::SignIn(result) => Self::SignIn {
                result: result.into(),
            },
            C::RetryPhoneEntry => Self::RetryPhoneEntry,
            C::Bind(result) => Self::Bind {
                result: result.into(),
            },
            C::RetryBinding => Self::RetryBinding,
            C::PermissionDecision { granted } => Self::PermissionDecision { granted },
            C::AutoUpdateConfirmed => Self::AutoUpdateConfirmed,
            C::SessionInvalidated { role } => Self::SessionInvalidated { role: role.into() },
            C::BelowMinVersionDetected => Self::BelowMinVersionDetected,
        }
    }
}

// ── Exported functions (the state-machine surface) ───────────────────────────────────────

/// Decide launch routing from whether the device holds a live session (ADR-0016 D2).
#[uniffi::export]
pub fn launch(has_valid_session: bool) -> LaunchDecision {
    core_auth::launch(has_valid_session).into()
}

/// Apply an onboarding event, returning the next state (the core's transition table, P4).
#[uniffi::export]
pub fn on_event(state: OnboardingState, event: OnboardingEvent) -> OnboardingState {
    core_auth::OnboardingState::from(state)
        .on_event(event.into())
        .into()
}

/// Whether a state is terminal — onboarding does not advance past it on its own.
#[uniffi::export]
pub fn is_terminal(state: OnboardingState) -> bool {
    core_auth::OnboardingState::from(state).is_terminal()
}

/// Whether the `Offline` overlay may be shown over this state (`PhoneEntry`/`DeviceBinding`).
#[uniffi::export]
pub fn allows_offline_overlay(state: OnboardingState) -> bool {
    core_auth::OnboardingState::from(state).allows_offline_overlay()
}

/// Where an invalidated session routes (AC15/AC18): Rider → calm help; Driver/Admin → re-auth.
#[uniffi::export]
pub fn reauth_state_for(role: Role) -> OnboardingState {
    core_auth::reauth_state_for(role.into()).into()
}

/// Whether a declined notification permission should record the non-PII admin flag (AC14).
#[uniffi::export]
pub fn should_flag_notifications_off(granted: bool) -> bool {
    core_auth::should_flag_notifications_off(granted)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `Role` round-trips mirror ⇄ core (exhaustiveness is compile-checked; this asserts
    /// the mapping is also an identity).
    #[test]
    fn role_round_trips_all_variants() {
        for r in [Role::Rider, Role::Driver, Role::Admin] {
            let core: core_domain::Role = r.into();
            assert_eq!(Role::from(core), r);
        }
    }

    /// Every `OnboardingState` round-trips mirror ⇄ core.
    #[test]
    fn onboarding_state_round_trips_all_variants() {
        for s in [
            OnboardingState::FreshInstall,
            OnboardingState::PhoneEntry,
            OnboardingState::DeviceBinding,
            OnboardingState::Permissions,
            OnboardingState::AutoUpdateStep,
            OnboardingState::Complete,
            OnboardingState::PhoneNotOnFile,
            OnboardingState::BindingFailed,
            OnboardingState::BelowMinVersion,
            OnboardingState::NeedsReauthHelp,
        ] {
            let core: core_auth::OnboardingState = s.into();
            assert_eq!(OnboardingState::from(core), s);
        }
    }

    #[test]
    fn launch_decision_and_results_round_trip() {
        for d in [LaunchDecision::Onboard, LaunchDecision::Resume] {
            let core: core_auth::LaunchDecision = d.into();
            assert_eq!(LaunchDecision::from(core), d);
        }
        for r in [
            SignInResult::MemberMatched,
            SignInResult::PhoneNotOnFile,
            SignInResult::BelowMinVersion,
        ] {
            let core: core_auth::SignInResult = r.into();
            assert_eq!(SignInResult::from(core), r);
        }
        for r in [BindResult::Bound, BindResult::Failed] {
            let core: core_auth::BindResult = r.into();
            assert_eq!(BindResult::from(core), r);
        }
    }

    /// Every `OnboardingEvent` (incl. data variants) round-trips mirror ⇄ core.
    #[test]
    fn onboarding_event_round_trips_all_variants() {
        for e in [
            OnboardingEvent::BeginSignIn,
            OnboardingEvent::SignIn {
                result: SignInResult::MemberMatched,
            },
            OnboardingEvent::SignIn {
                result: SignInResult::PhoneNotOnFile,
            },
            OnboardingEvent::SignIn {
                result: SignInResult::BelowMinVersion,
            },
            OnboardingEvent::RetryPhoneEntry,
            OnboardingEvent::Bind {
                result: BindResult::Bound,
            },
            OnboardingEvent::Bind {
                result: BindResult::Failed,
            },
            OnboardingEvent::RetryBinding,
            OnboardingEvent::PermissionDecision { granted: true },
            OnboardingEvent::PermissionDecision { granted: false },
            OnboardingEvent::AutoUpdateConfirmed,
            OnboardingEvent::SessionInvalidated { role: Role::Rider },
            OnboardingEvent::SessionInvalidated { role: Role::Driver },
            OnboardingEvent::BelowMinVersionDetected,
        ] {
            let core: core_auth::OnboardingEvent = e.into();
            assert_eq!(OnboardingEvent::from(core), e);
        }
    }

    /// The exported wrappers delegate to the core graph faithfully (happy path + routing),
    /// so the FFI surface — not just the conversions — is exercised on the host.
    #[test]
    fn exported_wrappers_drive_the_core_graph() {
        assert_eq!(launch(false), LaunchDecision::Onboard);
        assert_eq!(launch(true), LaunchDecision::Resume);

        let mut s = OnboardingState::FreshInstall;
        s = on_event(s, OnboardingEvent::BeginSignIn);
        assert_eq!(s, OnboardingState::PhoneEntry);
        s = on_event(
            s,
            OnboardingEvent::SignIn {
                result: SignInResult::MemberMatched,
            },
        );
        assert_eq!(s, OnboardingState::DeviceBinding);
        s = on_event(
            s,
            OnboardingEvent::Bind {
                result: BindResult::Bound,
            },
        );
        assert_eq!(s, OnboardingState::Permissions);
        s = on_event(s, OnboardingEvent::PermissionDecision { granted: false });
        assert_eq!(s, OnboardingState::AutoUpdateStep);
        s = on_event(s, OnboardingEvent::AutoUpdateConfirmed);
        assert_eq!(s, OnboardingState::Complete);
        assert!(is_terminal(s));

        // Cross-cutting routing.
        assert_eq!(
            reauth_state_for(Role::Rider),
            OnboardingState::NeedsReauthHelp
        );
        assert_eq!(reauth_state_for(Role::Driver), OnboardingState::PhoneEntry);
        assert!(allows_offline_overlay(OnboardingState::PhoneEntry));
        assert!(!allows_offline_overlay(OnboardingState::Permissions));
        assert!(should_flag_notifications_off(false));
        assert!(!should_flag_notifications_off(true));
    }
}
