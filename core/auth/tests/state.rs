//! Onboarding state-machine tests, including the AC8 (below-min) and AC15 (invalidated
//! session) routing decisions, grounded against the golden fixtures (`fixtures/auth/**`).

use boundless_auth::{
    evaluate_version, reauth_state_for, should_flag_notifications_off, BindResult, OnboardingEvent,
    OnboardingState, SignInResult, VersionRequirement, VersionVerdict, SESSION_INVALIDATED_CODE,
};
use boundless_domain::{AppVersion, Role};
use std::path::{Path, PathBuf};

fn fixtures_dir() -> PathBuf {
    // boundless-auth's manifest dir is core/auth; the golden fixtures live at the repo root.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn read_fixture(rel: &str) -> serde_json::Value {
    let raw = std::fs::read_to_string(fixtures_dir().join(rel))
        .unwrap_or_else(|e| panic!("read fixture {rel}: {e}"));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse fixture {rel}: {e}"))
}

#[test]
fn ac8_below_min_version_routes_to_degradation() {
    // Replay fixtures/auth/below_min_version.json: the reported version is below
    // client_min_version → the core verdict is BelowMinimum and any state degrades.
    let v = read_fixture("auth/below_min_version.json");
    assert_eq!(v["error_code"], "AUTH_BELOW_MIN_VERSION");
    let reported: AppVersion = v["reported_client_version"]["app_version"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let min: AppVersion = v["client_min_version"].as_str().unwrap().parse().unwrap();
    let recommended: AppVersion = v["client_recommended_version"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    let verdict = evaluate_version(&reported, &VersionRequirement::new(min, recommended));
    assert_eq!(verdict, VersionVerdict::BelowMinimum);
    assert_eq!(verdict.error_code(), Some("AUTH_BELOW_MIN_VERSION"));

    // Reachable from any auth response / WS handshake — including a returning, completed
    // session that never entered PhoneEntry (O4).
    assert_eq!(
        OnboardingState::PhoneEntry.on_event(OnboardingEvent::BelowMinVersionDetected),
        OnboardingState::BelowMinVersion
    );
    assert_eq!(
        OnboardingState::Complete.on_event(OnboardingEvent::BelowMinVersionDetected),
        OnboardingState::BelowMinVersion
    );
    assert!(OnboardingState::BelowMinVersion.is_terminal());
}

#[test]
fn ac15_invalidated_rider_routes_to_needs_reauth_help() {
    // Replay fixtures/auth/needs_reauth_help.json: a Rider's invalidated session routes to
    // the calm help screen (never a form); the fixture's routes_to must match the core call.
    let v = read_fixture("auth/needs_reauth_help.json");
    assert_eq!(v["error_code"], "AUTH_SESSION_INVALIDATED");
    assert_eq!(v["role"], "rider");
    assert_eq!(v["routes_to"], "NeedsReauthHelp");
    assert_eq!(SESSION_INVALIDATED_CODE, "AUTH_SESSION_INVALIDATED");

    assert_eq!(
        reauth_state_for(Role::Rider),
        OnboardingState::NeedsReauthHelp
    );
    assert!(OnboardingState::NeedsReauthHelp.is_terminal());

    // Reachable from any state; a Driver instead routes to interactive re-auth (PhoneEntry).
    assert_eq!(
        OnboardingState::Complete
            .on_event(OnboardingEvent::SessionInvalidated { role: Role::Rider }),
        OnboardingState::NeedsReauthHelp
    );
    assert_eq!(
        OnboardingState::Complete
            .on_event(OnboardingEvent::SessionInvalidated { role: Role::Driver }),
        OnboardingState::PhoneEntry
    );
}

#[test]
fn phone_not_on_file_loops_back_to_entry() {
    let s =
        OnboardingState::PhoneEntry.on_event(OnboardingEvent::SignIn(SignInResult::PhoneNotOnFile));
    assert_eq!(s, OnboardingState::PhoneNotOnFile);
    assert_eq!(
        s.on_event(OnboardingEvent::RetryPhoneEntry),
        OnboardingState::PhoneEntry
    );
}

#[test]
fn binding_failed_loops_back_to_binding() {
    let s = OnboardingState::DeviceBinding.on_event(OnboardingEvent::Bind(BindResult::Failed));
    assert_eq!(s, OnboardingState::BindingFailed);
    assert_eq!(
        s.on_event(OnboardingEvent::RetryBinding),
        OnboardingState::DeviceBinding
    );
}

#[test]
fn offline_overlay_only_on_signin_steps_and_does_not_transition() {
    // Offline is an overlay on PhoneEntry/DeviceBinding, not a node — the state is unchanged
    // while connectivity is absent (the network action is deferred, then resumed).
    assert!(OnboardingState::PhoneEntry.allows_offline_overlay());
    assert!(OnboardingState::DeviceBinding.allows_offline_overlay());
    assert!(!OnboardingState::Permissions.allows_offline_overlay());
    assert!(!OnboardingState::AutoUpdateStep.allows_offline_overlay());
    assert!(!OnboardingState::Complete.allows_offline_overlay());
}

#[test]
fn permission_decline_still_advances_and_flags() {
    // AC14: onboarding never blocks/scolds — both decisions advance to the auto-update step;
    // a decline records the non-PII "notifications not enabled" admin flag (server-side).
    assert_eq!(
        OnboardingState::Permissions
            .on_event(OnboardingEvent::PermissionDecision { granted: true }),
        OnboardingState::AutoUpdateStep
    );
    assert_eq!(
        OnboardingState::Permissions
            .on_event(OnboardingEvent::PermissionDecision { granted: false }),
        OnboardingState::AutoUpdateStep
    );
    assert!(should_flag_notifications_off(false));
    assert!(!should_flag_notifications_off(true));
}

#[test]
fn spurious_or_out_of_order_event_is_a_noop() {
    // An event with no defined transition for the current state leaves it unchanged.
    assert_eq!(
        OnboardingState::PhoneEntry.on_event(OnboardingEvent::AutoUpdateConfirmed),
        OnboardingState::PhoneEntry
    );
    assert_eq!(
        OnboardingState::Complete.on_event(OnboardingEvent::BeginSignIn),
        OnboardingState::Complete
    );
}

#[test]
fn signin_interpretation_prioritizes_version_handshake() {
    // O4: a below-minimum handshake degrades regardless of whether the phone matched.
    assert_eq!(
        SignInResult::from_lookup(true, VersionVerdict::BelowMinimum),
        SignInResult::BelowMinVersion
    );
    assert_eq!(
        SignInResult::from_lookup(false, VersionVerdict::BelowMinimum),
        SignInResult::BelowMinVersion
    );
    assert_eq!(
        SignInResult::from_lookup(true, VersionVerdict::Supported),
        SignInResult::MemberMatched
    );
    assert_eq!(
        SignInResult::from_lookup(false, VersionVerdict::Supported),
        SignInResult::PhoneNotOnFile
    );
}

#[test]
fn bind_interpretation_collapses_rejections() {
    use boundless_auth::OnboardingCodeVerdict::*;
    assert_eq!(BindResult::from_verdict(Accepted), BindResult::Bound);
    for v in [Invalid, Expired, Consumed, RateLimited] {
        assert_eq!(BindResult::from_verdict(v), BindResult::Failed);
    }
}

#[test]
fn signin_ok_fixture_routes_to_device_binding() {
    // Parity anchor: the happy-path sign-in fixture (replayed across platforms, plan §5) must
    // agree with the core's interpretation — a matched lookup at a supported version routes
    // PhoneEntry → DeviceBinding, which is exactly what the fixture's next_step records.
    let v = read_fixture("auth/signin_ok.json");
    assert_eq!(v["outcome"], "member_matched");
    assert_eq!(v["next_step"], "device_binding");

    let min: AppVersion = v["client_min_version"].as_str().unwrap().parse().unwrap();
    let recommended: AppVersion = v["client_recommended_version"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    // A fully-current client (reporting the recommended version) that matched.
    let verdict = evaluate_version(&recommended, &VersionRequirement::new(min, recommended));
    let result = SignInResult::from_lookup(true, verdict);
    assert_eq!(result, SignInResult::MemberMatched);
    assert_eq!(
        OnboardingState::PhoneEntry.on_event(OnboardingEvent::SignIn(result)),
        OnboardingState::DeviceBinding
    );
}

// --- Exhaustive transition-graph coverage --------------------------------------------------
//
// The three tests below pin the *whole* graph, not a handful of edges, so a future refactor
// that drops an edge, mis-routes a cross-cutting override, or turns an undefined edge into an
// accidental transition fails here. The expected edges are written from the spec table
// independently of the implementation.

/// Every `OnboardingState` variant.
fn all_states() -> [OnboardingState; 10] {
    [
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
    ]
}

/// Every non-cross-cutting event (the table-driven ones). The cross-cutting overrides
/// (`BelowMinVersionDetected`, `SessionInvalidated`) are tested separately.
fn all_non_crosscutting_events() -> Vec<OnboardingEvent> {
    vec![
        OnboardingEvent::BeginSignIn,
        OnboardingEvent::SignIn(SignInResult::MemberMatched),
        OnboardingEvent::SignIn(SignInResult::PhoneNotOnFile),
        OnboardingEvent::SignIn(SignInResult::BelowMinVersion),
        OnboardingEvent::RetryPhoneEntry,
        OnboardingEvent::Bind(BindResult::Bound),
        OnboardingEvent::Bind(BindResult::Failed),
        OnboardingEvent::RetryBinding,
        OnboardingEvent::PermissionDecision { granted: true },
        OnboardingEvent::PermissionDecision { granted: false },
        OnboardingEvent::AutoUpdateConfirmed,
    ]
}

/// The spec's transition table (`spec.md` "States and transitions"), as `(from, event, to)`.
fn defined_transitions() -> Vec<(OnboardingState, OnboardingEvent, OnboardingState)> {
    use OnboardingEvent as E;
    use OnboardingState as S;
    vec![
        (S::FreshInstall, E::BeginSignIn, S::PhoneEntry),
        (
            S::PhoneEntry,
            E::SignIn(SignInResult::MemberMatched),
            S::DeviceBinding,
        ),
        (
            S::PhoneEntry,
            E::SignIn(SignInResult::PhoneNotOnFile),
            S::PhoneNotOnFile,
        ),
        (
            S::PhoneEntry,
            E::SignIn(SignInResult::BelowMinVersion),
            S::BelowMinVersion,
        ),
        (S::PhoneNotOnFile, E::RetryPhoneEntry, S::PhoneEntry),
        (S::DeviceBinding, E::Bind(BindResult::Bound), S::Permissions),
        (
            S::DeviceBinding,
            E::Bind(BindResult::Failed),
            S::BindingFailed,
        ),
        (S::BindingFailed, E::RetryBinding, S::DeviceBinding),
        (
            S::Permissions,
            E::PermissionDecision { granted: true },
            S::AutoUpdateStep,
        ),
        (
            S::Permissions,
            E::PermissionDecision { granted: false },
            S::AutoUpdateStep,
        ),
        (S::AutoUpdateStep, E::AutoUpdateConfirmed, S::Complete),
    ]
}

#[test]
fn defined_transitions_match_the_spec_table() {
    for (from, event, to) in defined_transitions() {
        assert_eq!(
            from.on_event(event),
            to,
            "{from:?} --{event:?}--> expected {to:?}"
        );
    }
}

#[test]
fn undefined_non_crosscutting_events_are_noops() {
    let defined = defined_transitions();
    for from in all_states() {
        for event in all_non_crosscutting_events() {
            let is_defined = defined.iter().any(|(s, e, _)| *s == from && *e == event);
            if !is_defined {
                assert_eq!(
                    from.on_event(event),
                    from,
                    "{from:?} --{event:?}--> should be a no-op (stay), but moved"
                );
            }
        }
    }
}

#[test]
fn crosscutting_events_override_from_every_state() {
    for from in all_states() {
        // Below-min degrades from anywhere (O4 — any auth response / WS handshake).
        assert_eq!(
            from.on_event(OnboardingEvent::BelowMinVersionDetected),
            OnboardingState::BelowMinVersion,
            "BelowMinVersionDetected from {from:?}"
        );
        // Rider invalidation → help screen (never a form); Driver/Admin → interactive re-auth.
        assert_eq!(
            from.on_event(OnboardingEvent::SessionInvalidated { role: Role::Rider }),
            OnboardingState::NeedsReauthHelp,
            "SessionInvalidated(Rider) from {from:?}"
        );
        for role in [Role::Driver, Role::Admin] {
            assert_eq!(
                from.on_event(OnboardingEvent::SessionInvalidated { role }),
                OnboardingState::PhoneEntry,
                "SessionInvalidated({role:?}) from {from:?}"
            );
        }
    }
}

#[test]
fn is_terminal_partitions_states_exactly() {
    for s in all_states() {
        let expected = matches!(
            s,
            OnboardingState::Complete
                | OnboardingState::BelowMinVersion
                | OnboardingState::NeedsReauthHelp
        );
        assert_eq!(s.is_terminal(), expected, "is_terminal({s:?})");
    }
}
