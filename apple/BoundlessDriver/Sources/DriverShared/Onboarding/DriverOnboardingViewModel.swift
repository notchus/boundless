import BoundlessKit
import Observation
import RiderShared

/// Drives the **Driver** onboarding flow. It composes the shared `OnboardingViewModel` with
/// `role: .driver` — so every transition is still the core's decision via `BoundlessKit` (P4 /
/// ADR-0001), never re-implemented here — and adds only the two Driver-specific concerns the shared
/// model doesn't carry: the captured Recovery Code (for the AC19 capture screen) and the fact that an
/// invalidated Driver session routes to interactive re-auth at `PhoneEntry` (vs the Rider's calm
/// `NeedsReauthHelp`). Side effects stay behind the injected protocols, so the flow is deterministic
/// in tests.
@MainActor
@Observable
public final class DriverOnboardingViewModel {
    private let core: OnboardingViewModel
    private let recovery: RecoveryCodeProviding

    /// Set when an invalidated session routed this Driver back to `PhoneEntry`, so the router shows
    /// the re-auth variant (`auth.signin_again`) rather than the fresh sign-in. The *target state* is
    /// still the core's decision (`reauth_state_for(.driver)`); this only records that it happened.
    public private(set) var reauthRequested = false

    public init(
        hasValidSession: Bool,
        networking: OnboardingNetworking,
        notifications: NotificationPermissionRequesting,
        manifest: ManifestProviding,
        recovery: RecoveryCodeProviding
    ) {
        self.core = OnboardingViewModel(
            role: .driver,
            hasValidSession: hasValidSession,
            networking: networking,
            notifications: notifications,
            manifest: manifest
        )
        self.recovery = recovery
    }

    // MARK: Shared state (forwarded from the core view model)

    public var state: OnboardingState { core.state }
    public var adminName: String? { core.adminName }
    public var notificationsFlaggedOff: Bool { core.notificationsFlaggedOff }

    /// The one-time Recovery Code to display on the capture screen, or `nil` if not captured yet
    /// (the router then skips the capture — never an empty-code screen, never a block).
    public var recoveryCode: String? { recovery.recoveryCode }

    // MARK: Events — each delegates the transition to the core (never decided here, P4)

    public func begin() { core.begin() }
    public func submitPhone(_ phone: String) async { await core.submitPhone(phone) }
    public func submitCode(_ code: String) async { await core.submitCode(code) }
    public func decideNotifications(allow: Bool) async { await core.decideNotifications(allow: allow) }
    public func confirmAutoUpdate() { core.confirmAutoUpdate() }
    public func belowMinVersionDetected() { core.belowMinVersionDetected() }

    /// A previously-valid Driver session was invalidated mid-life. The core routes a Driver to
    /// `PhoneEntry` for interactive re-auth (AC15/AC18); we record that so the router leads that
    /// screen with `auth.signin_again`.
    public func sessionInvalidated() {
        reauthRequested = true
        core.sessionInvalidated()
    }
}
