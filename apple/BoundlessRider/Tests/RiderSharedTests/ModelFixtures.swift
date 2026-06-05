import BoundlessKit
import SwiftUI

@testable import RiderShared

/// Builds an onboarding view model wired to stubs.
@MainActor
func makeOnboardingVM(
    role: Role = .rider,
    signIn: SignInResult = .memberMatched,
    bind: BindResult = .bound,
    granted: Bool = true,
    hasValidSession: Bool = false,
    adminName: String? = Fixtures.adminName
) -> OnboardingViewModel {
    OnboardingViewModel(
        role: role,
        hasValidSession: hasValidSession,
        networking: StubNetworking(signIn: signIn, bind: bind),
        notifications: StubNotifications(granted: granted),
        manifest: StubManifest(adminName: adminName)
    )
}

/// Drives a fresh view model through the happy path up to (and stopping at) the Permissions step.
@MainActor
func advanceToPermissions(_ vm: OnboardingViewModel) async {
    vm.begin()
    await vm.submitPhone("5551234567")
    await vm.submitCode("123456")
}

/// Every Rider onboarding screen model, built with constant bindings + no-op callbacks. Used by the
/// no-signup-route (AC1(b)) and VoiceOver-order (AC11) tests to sweep across all screens at once.
enum ScreenFixtures {
    static func allModels(adminName: String = Fixtures.adminName) -> [(name: String, model: OnboardingScreenModel)] {
        [
            ("helperIntro", RiderOnboardingScreens.helperIntro(onContinue: {})),
            ("phoneEntry", RiderOnboardingScreens.phoneEntry(text: .constant(""), onContinue: {})),
            ("phoneEntryOffline", RiderOnboardingScreens.phoneEntry(text: .constant(""), isOffline: true, onContinue: {})),
            ("phoneNotOnFile", RiderOnboardingScreens.phoneNotOnFile(text: .constant(""), adminName: adminName, onTryAgain: {})),
            ("deviceBinding", RiderOnboardingScreens.deviceBinding(text: .constant(""), adminName: adminName, onContinue: {})),
            ("bindingFailed", RiderOnboardingScreens.bindingFailed(text: .constant(""), adminName: adminName, onTryAgain: {})),
            ("permissions", RiderOnboardingScreens.permissions(onAllow: {}, onDecline: {})),
            ("permissionsDeclined", RiderOnboardingScreens.permissionsDeclined(adminName: adminName, onContinue: {})),
            ("autoUpdateStep", RiderOnboardingScreens.autoUpdateStep(onContinue: {})),
            ("autoUpdateEnabled", RiderOnboardingScreens.autoUpdateEnabled(onContinue: {})),
            ("belowMinVersionNamed", RiderOnboardingScreens.calmHelp(adminName: adminName)),
            ("belowMinVersionGeneric", RiderOnboardingScreens.calmHelp(adminName: nil)),
            // Name-less variants (no manifest cached): the four name-bearing screens must render a
            // generic fallback, never an empty name slot (reviewer T11 confirmed finding).
            ("phoneNotOnFileNil", RiderOnboardingScreens.phoneNotOnFile(text: .constant(""), adminName: nil, onTryAgain: {})),
            ("deviceBindingNil", RiderOnboardingScreens.deviceBinding(text: .constant(""), adminName: nil, onContinue: {})),
            ("bindingFailedNil", RiderOnboardingScreens.bindingFailed(text: .constant(""), adminName: nil, onTryAgain: {})),
            ("permissionsDeclinedNil", RiderOnboardingScreens.permissionsDeclined(adminName: nil, onContinue: {})),
        ]
    }
}
