import BoundlessKit
import RiderShared
import SwiftUI

@testable import DriverShared

/// Builds a Driver onboarding view model wired to stubs.
@MainActor
func makeDriverVM(
    signIn: SignInResult = .memberMatched,
    bind: BindResult = .bound,
    granted: Bool = true,
    hasValidSession: Bool = false,
    adminName: String? = Fixtures.adminName,
    recoveryCode: String? = Fixtures.recoveryCode
) -> DriverOnboardingViewModel {
    DriverOnboardingViewModel(
        hasValidSession: hasValidSession,
        networking: StubNetworking(signIn: signIn, bind: bind),
        notifications: StubNotifications(granted: granted),
        manifest: StubManifest(adminName: adminName),
        recovery: StubRecovery(recoveryCode: recoveryCode)
    )
}

/// Drives a fresh Driver view model through the happy path up to (and stopping at) the core's
/// `Permissions` state — where the Driver router first shows the Recovery Code capture interstitial.
@MainActor
func advanceToPermissions(_ vm: DriverOnboardingViewModel) async {
    vm.begin()
    await vm.submitPhone("5551234567")
    await vm.submitCode("123456")
}

/// Every Driver onboarding screen model the router can show — the Driver-specific screens plus the
/// role-neutral ones it reuses from `RiderShared`. Built with constant bindings + no-op callbacks.
/// Used by the no-signup-route (AC1(b)), VoiceOver-order (AC11) and snapshot (AC11) tests to sweep
/// across the whole Driver surface at once.
enum DriverScreenFixtures {
    static func allModels(
        adminName: String = Fixtures.adminName
    ) -> [(name: String, model: OnboardingScreenModel)] {
        [
            // ── Driver-specific ───────────────────────────────────────────────────────────
            ("driverIntro", DriverOnboardingScreens.driverIntro(onContinue: {})),
            ("reAuthPhoneEntry", DriverOnboardingScreens.reAuthPhoneEntry(text: .constant("")) {}),
            ("recoveryCodeCapture", DriverOnboardingScreens.recoveryCodeCapture(code: Fixtures.recoveryCode) {}),
            // ── Reused role-neutral steps (rendered identically to the Rider; the Driver app's own
            //    baselines independently close AC11 for this platform) ───────────────────────
            ("phoneEntry", RiderOnboardingScreens.phoneEntry(text: .constant(""), onContinue: {})),
            ("phoneEntryOffline", RiderOnboardingScreens.phoneEntry(text: .constant(""), isOffline: true, onContinue: {})),
            ("phoneNotOnFile", RiderOnboardingScreens.phoneNotOnFile(text: .constant(""), adminName: adminName, onTryAgain: {})),
            ("phoneNotOnFileNil", RiderOnboardingScreens.phoneNotOnFile(text: .constant(""), adminName: nil, onTryAgain: {})),
            ("deviceBinding", RiderOnboardingScreens.deviceBinding(text: .constant(""), adminName: adminName, onContinue: {})),
            ("deviceBindingOffline", RiderOnboardingScreens.deviceBinding(text: .constant(""), adminName: adminName, isOffline: true, onContinue: {})),
            ("bindingFailed", RiderOnboardingScreens.bindingFailed(text: .constant(""), adminName: adminName, onTryAgain: {})),
            ("bindingFailedNil", RiderOnboardingScreens.bindingFailed(text: .constant(""), adminName: nil, onTryAgain: {})),
            ("permissions", RiderOnboardingScreens.permissions(onAllow: {}, onDecline: {})),
            ("permissionsDeclined", RiderOnboardingScreens.permissionsDeclined(adminName: adminName, onContinue: {})),
            ("permissionsDeclinedNil", RiderOnboardingScreens.permissionsDeclined(adminName: nil, onContinue: {})),
            ("autoUpdateStep", RiderOnboardingScreens.autoUpdateStep(onContinue: {})),
            ("autoUpdateEnabled", RiderOnboardingScreens.autoUpdateEnabled(onContinue: {})),
            ("belowMinVersionNamed", RiderOnboardingScreens.calmHelp(adminName: adminName)),
            ("belowMinVersionGeneric", RiderOnboardingScreens.calmHelp(adminName: nil)),
        ]
    }
}
