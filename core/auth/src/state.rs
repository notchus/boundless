//! The device-side onboarding state machine (spec 001 "States and transitions").
//!
//! Defined once in the core (P4) so SwiftUI, Compose, and every other client transition
//! **identically** — a platform renders a state, it does not decide it. The clients
//! (T11–T15) own the pixels; the server (T07) owns enforcement and the rate-limited admin
//! alerts; this module owns the *graph*.
//!
//! Two rows of the spec's table are deliberately **not** states:
//! - **`Offline`** is an *overlay* on `PhoneEntry`/`DeviceBinding`, not a node: losing
//!   connectivity does not transition, it defers the network action (lookup/bind/manifest)
//!   until reconnect and resumes the same step. Binding cannot complete offline (the
//!   Onboarding Code is server-validated). See [`OnboardingState::allows_offline_overlay`].
//! - **`ManifestFailReturning`** is a returning-device outcome, not a screen: a device with
//!   a live session [`resumes`](launch) straight to the primary surface, and when its
//!   manifest fetch/verify fails it keeps the previously-cached manifest (core::crypto's
//!   `ManifestDecision::KeepCached`, T03) and never blocks. It is the pairing of
//!   [`LaunchDecision::Resume`] with that cached-manifest decision, not a distinct node.

use boundless_domain::Role;
use serde::{Deserialize, Serialize};

use crate::code::OnboardingCodeVerdict;
use crate::version::VersionVerdict;

/// A screen in the onboarding flow. Terminal calm screens (`BelowMinVersion`,
/// `NeedsReauthHelp`) and `Complete` route out of onboarding; see [`OnboardingState::is_terminal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    /// surface. Completion is silent (no "all set" screen — voice-and-tone).
    Complete,
    /// Phone-lookup miss; returns to `PhoneEntry`. Never reveals whether a number exists.
    PhoneNotOnFile,
    /// Onboarding Code invalid/expired/consumed/rate-limited; returns to `DeviceBinding`.
    BindingFailed,
    /// Client below `client_min_version` — the calm degradation screen, no "Update Now"
    /// (O4/O8). Reachable from **any** auth response / WS handshake.
    BelowMinVersion,
    /// A previously-valid **Rider** session was invalidated with no helper present — the
    /// calm help screen, never a sign-in form (AC15/P10). A **Driver** routes to `PhoneEntry`
    /// instead (see [`reauth_state_for`]).
    NeedsReauthHelp,
}

impl OnboardingState {
    /// Whether the `Offline` overlay may be shown over this state (spec: an overlay on
    /// `PhoneEntry`/`DeviceBinding`, not a node). Connectivity loss elsewhere is not modeled
    /// as an overlay.
    pub const fn allows_offline_overlay(self) -> bool {
        matches!(self, Self::PhoneEntry | Self::DeviceBinding)
    }

    /// Whether this is a terminal screen — onboarding does not advance past it on its own.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Complete | Self::BelowMinVersion | Self::NeedsReauthHelp
        )
    }
}

/// Where a launch routes before any onboarding step (spec `FreshInstall` row + the
/// returning-device edge cases).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaunchDecision {
    /// No live session → enter onboarding at [`OnboardingState::FreshInstall`].
    Onboard,
    /// A live session exists → resume straight to the role's primary surface. Manifest
    /// cached-vs-bundled is decided by core::crypto (`decide_manifest`, T03); a verify
    /// failure here keeps the cache and never blocks (the `ManifestFailReturning` behavior).
    Resume,
}

/// Decide launch routing from whether the device holds a live session (ADR-0016 D2: sessions
/// are indefinite, so a returning device resumes without re-auth).
pub const fn launch(has_valid_session: bool) -> LaunchDecision {
    if has_valid_session {
        LaunchDecision::Resume
    } else {
        LaunchDecision::Onboard
    }
}

/// The interpreted result of a `/api/auth/signin` response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignInResult {
    /// Phone matched and version is supported → proceed to device binding.
    MemberMatched,
    /// Phone-lookup miss.
    PhoneNotOnFile,
    /// The handshake reported the client below `client_min_version` (O4).
    BelowMinVersion,
}

impl SignInResult {
    /// Interpret a sign-in response: the **version handshake is checked first** (O4), so a
    /// below-minimum client degrades regardless of whether the phone matched; otherwise the
    /// phone-match outcome decides.
    pub const fn from_lookup(matched: bool, verdict: VersionVerdict) -> Self {
        if verdict.is_below_minimum() {
            Self::BelowMinVersion
        } else if matched {
            Self::MemberMatched
        } else {
            Self::PhoneNotOnFile
        }
    }
}

/// The interpreted result of a `/api/auth/bind-device` response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindResult {
    /// The Onboarding Code was accepted; the device is bound and a session issued.
    Bound,
    /// The Onboarding Code was rejected (invalid/expired/consumed/rate-limited) — all route
    /// to the same `BindingFailed` recovery screen.
    Failed,
}

impl BindResult {
    /// A bind succeeds only on an accepted verdict; every rejection collapses to `Failed`
    /// (the specific reason is carried separately as the error code).
    pub const fn from_verdict(verdict: OnboardingCodeVerdict) -> Self {
        if verdict.is_accepted() {
            Self::Bound
        } else {
            Self::Failed
        }
    }
}

/// An input to the onboarding state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnboardingEvent {
    /// Begin sign-in from a fresh install.
    BeginSignIn,
    /// A `/api/auth/signin` response was interpreted.
    SignIn(SignInResult),
    /// Retry from the `PhoneNotOnFile` banner.
    RetryPhoneEntry,
    /// A `/api/auth/bind-device` response was interpreted.
    Bind(BindResult),
    /// Retry from the `BindingFailed` recovery screen.
    RetryBinding,
    /// The notification-permission decision was recorded. Either way the flow advances
    /// (onboarding never blocks/scolds, AC14); on `granted == false` the server records a
    /// non-PII "notifications not enabled" admin flag — see [`should_flag_notifications_off`].
    PermissionDecision {
        /// Whether notification permission was granted.
        granted: bool,
    },
    /// The OS auto-update step (O3) was confirmed complete.
    AutoUpdateConfirmed,
    /// A previously-valid session was invalidated mid-life (admin revoke/logout, I4 device
    /// change, or deletion). Reachable from **any** state; routes per [`reauth_state_for`].
    SessionInvalidated {
        /// The member's role, which decides Rider (help screen) vs Driver (re-auth) routing.
        role: Role,
    },
    /// A below-`client_min_version` handshake was detected (O4). Reachable from **any** state
    /// — including a returning session that never entered `PhoneEntry`.
    BelowMinVersionDetected,
}

impl OnboardingState {
    /// Apply an event, returning the next state. Faithful to the spec's transition table.
    ///
    /// [`OnboardingEvent::BelowMinVersionDetected`] and [`OnboardingEvent::SessionInvalidated`]
    /// override from any state (they arrive on any auth response / WS handshake). Any other
    /// event that does not apply to the current state is a no-op (the state is unchanged) —
    /// a spurious or out-of-order event never corrupts the flow.
    pub fn on_event(self, event: OnboardingEvent) -> OnboardingState {
        use OnboardingEvent as E;
        use OnboardingState as S;

        // Cross-cutting events apply from any state.
        match event {
            E::BelowMinVersionDetected => return S::BelowMinVersion,
            E::SessionInvalidated { role } => return reauth_state_for(role),
            _ => {}
        }

        match (self, event) {
            (S::FreshInstall, E::BeginSignIn) => S::PhoneEntry,
            (S::PhoneEntry, E::SignIn(SignInResult::MemberMatched)) => S::DeviceBinding,
            (S::PhoneEntry, E::SignIn(SignInResult::PhoneNotOnFile)) => S::PhoneNotOnFile,
            (S::PhoneEntry, E::SignIn(SignInResult::BelowMinVersion)) => S::BelowMinVersion,
            (S::PhoneNotOnFile, E::RetryPhoneEntry) => S::PhoneEntry,
            (S::DeviceBinding, E::Bind(BindResult::Bound)) => S::Permissions,
            (S::DeviceBinding, E::Bind(BindResult::Failed)) => S::BindingFailed,
            (S::BindingFailed, E::RetryBinding) => S::DeviceBinding,
            (S::Permissions, E::PermissionDecision { .. }) => S::AutoUpdateStep,
            (S::AutoUpdateStep, E::AutoUpdateConfirmed) => S::Complete,
            // No defined transition for this (state, event): stay put.
            (state, _) => state,
        }
    }
}

/// Where an invalidated session routes (AC15/AC18). A **Rider** sees the calm
/// `NeedsReauthHelp` screen — never a sign-in form (P10); a **Driver** (and any non-Rider,
/// since Admins authenticate via WebAuthn on the web and hold their *member* role here) is
/// routed to interactive re-auth at `PhoneEntry`.
pub const fn reauth_state_for(role: Role) -> OnboardingState {
    match role {
        Role::Rider => OnboardingState::NeedsReauthHelp,
        Role::Driver | Role::Admin => OnboardingState::PhoneEntry,
    }
}

/// Whether a declined notification permission should record the non-PII "notifications not
/// enabled" admin flag (AC14). The flag itself is emitted server-side (T07); the flow always
/// advances regardless.
pub const fn should_flag_notifications_off(granted: bool) -> bool {
    !granted
}

/// The stable error code emitted when a session is invalidated (`docs/error-codes.md`, P12).
/// The server emits it (one admin alert per member per day); the client routes per
/// [`reauth_state_for`].
pub const SESSION_INVALIDATED_CODE: &str = "AUTH_SESSION_INVALIDATED";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_traversal() {
        let mut s = OnboardingState::FreshInstall;
        s = s.on_event(OnboardingEvent::BeginSignIn);
        assert_eq!(s, OnboardingState::PhoneEntry);
        s = s.on_event(OnboardingEvent::SignIn(SignInResult::MemberMatched));
        assert_eq!(s, OnboardingState::DeviceBinding);
        s = s.on_event(OnboardingEvent::Bind(BindResult::Bound));
        assert_eq!(s, OnboardingState::Permissions);
        s = s.on_event(OnboardingEvent::PermissionDecision { granted: true });
        assert_eq!(s, OnboardingState::AutoUpdateStep);
        s = s.on_event(OnboardingEvent::AutoUpdateConfirmed);
        assert_eq!(s, OnboardingState::Complete);
        assert!(s.is_terminal());
    }

    #[test]
    fn launch_resumes_with_session() {
        assert_eq!(launch(true), LaunchDecision::Resume);
        assert_eq!(launch(false), LaunchDecision::Onboard);
    }

    #[test]
    fn reauth_routes_by_role() {
        assert_eq!(
            reauth_state_for(Role::Rider),
            OnboardingState::NeedsReauthHelp
        );
        assert_eq!(reauth_state_for(Role::Driver), OnboardingState::PhoneEntry);
    }
}
