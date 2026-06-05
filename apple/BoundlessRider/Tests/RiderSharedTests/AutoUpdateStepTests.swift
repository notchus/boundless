import XCTest

@testable import RiderShared

/// AC5 — the first-launch flow contains an auto-update step and a screen labeled "auto-update
/// enabled" (asserts O3). The snapshot leg is in `OnboardingSnapshotTests`; this asserts the copy
/// resolves and the confirmation screen carries the labeled, completed-state element.
final class AutoUpdateStepTests: XCTestCase {
    func testAutoUpdateEnabledScreenIsLabeled() {
        XCTAssertEqual(L10n.autoUpdateEnabled, "Automatic updates are on.")

        let model = RiderOnboardingScreens.autoUpdateEnabled(onContinue: {})
        // The "auto-update enabled" label is present as a completed-state confirmation element.
        XCTAssertTrue(model.elements.contains(.confirmation(L10n.autoUpdateEnabled)))
    }

    func testAutoUpdateStepPresentsTheStep() {
        XCTAssertEqual(L10n.autoUpdateStep, "Turn on automatic updates.")
        let model = RiderOnboardingScreens.autoUpdateStep(onContinue: {})
        XCTAssertTrue(model.elements.contains(.heading(L10n.autoUpdateStep)))
    }
}
