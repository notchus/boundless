import BoundlessKit
import Foundation

/// Requests OS notification permission. The real implementation (wrapping `UNUserNotificationCenter`,
/// aiming at Critical Alerts once the entitlement lands — DEFERRED) is part of the app shell; the
/// flow depends only on this protocol so it is testable with a stub.
@MainActor
public protocol NotificationPermissionRequesting {
    /// Returns whether notification permission was granted.
    func requestAuthorization() async -> Bool
}

/// Supplies the per-Group admin name from the signed KV manifest (ADR-0014), read from cache at
/// launch. The real fetch/verify is the deferred shell; `nil` → the name-less fallback copy is used.
@MainActor
public protocol ManifestProviding {
    var adminName: String? { get }
}

/// The sign-in / device-bind network boundary. The real implementation is the swift-openapi-generator
/// HTTP client (deferred — the deployable Worker does not exist yet, T07-shell-B). The view model
/// feeds the interpreted result straight into the core state machine; it never decides outcomes (P4).
@MainActor
public protocol OnboardingNetworking {
    func signIn(phone: String) async -> SignInResult
    func bindDevice(code: String) async -> BindResult
}

/// Drives the Rider onboarding flow. Holds the current `OnboardingState` and applies events through
/// the `core::auth` state machine exported by `BoundlessKit` — **every** transition is the core's
/// decision (`onEvent`), never re-implemented here (constitution P4 / ADR-0001). Side effects
/// (network, OS permission, manifest) are injected, so the whole flow is deterministic in tests.
@MainActor
@Observable
public final class OnboardingViewModel {
    public private(set) var state: OnboardingState

    /// Set when a declined (or unavailable) notification permission must be recorded as a non-PII
    /// admin flag (AC14). The CORE decides this (`BoundlessKit.shouldFlagNotificationsOff`), not the
    /// UI (P4); actually *sending* the flag to the server is the deferred shell.
    public private(set) var notificationsFlaggedOff = false

    public let role: Role

    private let networking: OnboardingNetworking
    private let notifications: NotificationPermissionRequesting
    private let manifest: ManifestProviding

    /// The admin's name for `{adminName}` copy, or `nil` → the name-less fallback (offline launch).
    public var adminName: String? { manifest.adminName }

    public init(
        role: Role,
        hasValidSession: Bool,
        networking: OnboardingNetworking,
        notifications: NotificationPermissionRequesting,
        manifest: ManifestProviding
    ) {
        self.role = role
        self.networking = networking
        self.notifications = notifications
        self.manifest = manifest
        // Launch routing is the core's decision (ADR-0016 D2): a live session resumes straight to
        // the primary surface (modelled here as `.complete`); otherwise onboarding begins.
        self.state = (launch(hasValidSession: hasValidSession) == .resume) ? .complete : .freshInstall
    }

    // MARK: Events — each delegates the transition to the core (never decided here, P4)

    public func begin() { apply(.beginSignIn) }

    public func submitPhone(_ phone: String) async {
        // From the PhoneNotOnFile banner, return to PhoneEntry first, then re-evaluate the lookup.
        if state == .phoneNotOnFile { apply(.retryPhoneEntry) }
        let result = await networking.signIn(phone: phone)
        apply(.signIn(result: result))
    }

    public func submitCode(_ code: String) async {
        if state == .bindingFailed { apply(.retryBinding) }
        let result = await networking.bindDevice(code: code)
        apply(.bind(result: result))
    }

    /// `allow == true` requests OS permission (showing the system prompt); `allow == false`
    /// ("Not now") skips it. Either way the flow advances and never scolds (AC14).
    public func decideNotifications(allow: Bool) async {
        let granted = allow ? await notifications.requestAuthorization() : false
        if shouldFlagNotificationsOff(granted: granted) {
            notificationsFlaggedOff = true
        }
        apply(.permissionDecision(granted: granted))
    }

    public func confirmAutoUpdate() { apply(.autoUpdateConfirmed) }

    /// A previously-valid session was invalidated mid-life (admin revoke / new-device / deletion).
    /// Routes per the core: a Rider → calm help; a Driver → interactive re-auth (AC15/AC18).
    public func sessionInvalidated() { apply(.sessionInvalidated(role: role)) }

    /// A below-`client_min_version` handshake was detected (O4) — reachable from any state.
    public func belowMinVersionDetected() { apply(.belowMinVersionDetected) }

    private func apply(_ event: OnboardingEvent) {
        state = onEvent(state: state, event: event)
    }
}
