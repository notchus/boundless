import SnapshotTesting
import SwiftUI
import XCTest

@testable import RiderShared

/// The a11y snapshot matrix (AC11): every Rider onboarding screen × {default, largest text, dark,
/// RTL}. Also closes the snapshot legs of AC5 (auto-update enabled), AC8 (calm below-min screen, no
/// CTA), AC14 (declined permission), AC15 (NeedsReauthHelp — no form). Strings come from the catalog
/// (P8); the screens render the core state machine (P4).
@MainActor
final class OnboardingSnapshotTests: XCTestCase {
    private let admin = Fixtures.adminName
    private let noop: () -> Void = {}

    func testHelperIntro() {
        assertA11ySnapshots(
            of: OnboardingScreenView(RiderOnboardingScreens.helperIntro(onContinue: noop)),
            named: "helperIntro"
        )
    }

    func testPhoneEntry() {
        assertA11ySnapshots(
            of: OnboardingScreenView(RiderOnboardingScreens.phoneEntry(text: .constant(""), onContinue: noop)),
            named: "phoneEntry"
        )
    }

    func testPhoneEntryOffline() {
        assertA11ySnapshots(
            of: OnboardingScreenView(
                RiderOnboardingScreens.phoneEntry(text: .constant(""), isOffline: true, onContinue: noop)
            ),
            named: "phoneEntry_offline"
        )
    }

    func testPhoneNotOnFile() {
        assertA11ySnapshots(
            of: OnboardingScreenView(
                RiderOnboardingScreens.phoneNotOnFile(text: .constant(""), adminName: admin, onTryAgain: noop)
            ),
            named: "phoneNotOnFile"
        )
    }

    func testDeviceBinding() {
        assertA11ySnapshots(
            of: OnboardingScreenView(
                RiderOnboardingScreens.deviceBinding(text: .constant(""), adminName: admin, onContinue: noop)
            ),
            named: "deviceBinding"
        )
    }

    func testDeviceBindingOffline() {
        assertA11ySnapshots(
            of: OnboardingScreenView(
                RiderOnboardingScreens.deviceBinding(
                    text: .constant(""), adminName: admin, isOffline: true, onContinue: noop
                )
            ),
            named: "deviceBinding_offline"
        )
    }

    /// Reflow proxy for the on-screen keyboard reducing the visible area (the OS keyboard itself
    /// cannot be captured in an offscreen hosting snapshot — a documented limitation, DEFERRED).
    func testDeviceBindingKeyboardInset() {
        assertA11ySnapshots(
            of: OnboardingScreenView(
                RiderOnboardingScreens.deviceBinding(text: .constant("1234"), adminName: admin, onContinue: noop)
            ),
            named: "deviceBinding_keyboardInset",
            layout: .fixed(width: 390, height: 420)
        )
    }

    func testBindingFailed() {
        assertA11ySnapshots(
            of: OnboardingScreenView(
                RiderOnboardingScreens.bindingFailed(text: .constant(""), adminName: admin, onTryAgain: noop)
            ),
            named: "bindingFailed"
        )
    }

    func testPermissions() {
        assertA11ySnapshots(
            of: OnboardingScreenView(RiderOnboardingScreens.permissions(onAllow: noop, onDecline: noop)),
            named: "permissions"
        )
    }

    func testPermissionsDeclined() {
        assertA11ySnapshots(
            of: OnboardingScreenView(
                RiderOnboardingScreens.permissionsDeclined(adminName: admin, onContinue: noop)
            ),
            named: "permissions_declined"
        )
    }

    func testAutoUpdateStep() {
        assertA11ySnapshots(
            of: OnboardingScreenView(RiderOnboardingScreens.autoUpdateStep(onContinue: noop)),
            named: "autoUpdateStep"
        )
    }

    func testAutoUpdateEnabled() {
        assertA11ySnapshots(
            of: OnboardingScreenView(RiderOnboardingScreens.autoUpdateEnabled(onContinue: noop)),
            named: "autoUpdateEnabled"
        )
    }

    func testBelowMinVersionNamed() {
        assertA11ySnapshots(
            of: OnboardingScreenView(RiderOnboardingScreens.calmHelp(adminName: admin)),
            named: "belowMinVersion_named"
        )
    }

    func testBelowMinVersionGeneric() {
        assertA11ySnapshots(
            of: OnboardingScreenView(RiderOnboardingScreens.calmHelp(adminName: nil)),
            named: "belowMinVersion_generic"
        )
    }

    func testNeedsReauthHelp() {
        // Same calm-screen pattern as below-min (AC15) — rendered from the same factory; the router
        // maps both `.belowMinVersion` and `.needsReauthHelp` here.
        assertA11ySnapshots(
            of: OnboardingScreenView(RiderOnboardingScreens.calmHelp(adminName: admin)),
            named: "needsReauthHelp"
        )
    }

    /// Silent completion: the hand-off placeholder, with no "all set" celebration (voice-and-tone).
    func testPrimarySurfacePlaceholder() {
        assertA11ySnapshots(of: PrimarySurfacePlaceholderView(), named: "primarySurface")
    }

    func testRiderSettings() {
        assertA11ySnapshots(
            of: NavigationStack { RiderSettingsView() },
            named: "riderSettings"
        )
    }
}
