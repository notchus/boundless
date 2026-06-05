import RiderShared
import XCTest

@testable import DriverShared

/// AC11 (reading-order leg) — every Driver screen has a complete, ordered VoiceOver reading:
/// non-empty labels, a leading header where the screen has a title, and the auto-update confirmation
/// announced as a completed *state*, not a button. swift-snapshot-testing has no a11y-tree strategy,
/// so this model-level assertion is the automatable order check; the recorded VoiceOver walkthrough
/// remains a manual checklist item (DEFERRED).
@MainActor
final class VoiceOverTraversalTests: XCTestCase {
    func testEveryScreenHasLabeledReadingOrder() {
        for (name, model) in DriverScreenFixtures.allModels() {
            let order = model.a11yReadingOrder
            XCTAssertFalse(order.isEmpty, "Screen '\(name)' has an empty reading order.")
            for descriptor in order {
                XCTAssertFalse(
                    descriptor.label.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                    "Screen '\(name)' has an unlabeled element (VoiceOver would read nothing)."
                )
                XCTAssertFalse(descriptor.traits.isEmpty, "Screen '\(name)' element has no trait.")
            }
        }
    }

    func testHeaderLeadsEachTitledScreen() {
        // Screens that lead with a title element must mark it as a header so VoiceOver lands there.
        let titled = [
            "driverIntro", "reAuthPhoneEntry", "recoveryCodeCapture",
            "phoneEntry", "deviceBinding", "permissions", "belowMinVersionNamed",
        ]
        let models = Dictionary(uniqueKeysWithValues: DriverScreenFixtures.allModels().map { ($0.name, $0.model) })
        for name in titled {
            let first = models[name]?.a11yReadingOrder.first
            XCTAssertEqual(first?.isHeader, true, "Screen '\(name)' should lead with a header.")
        }
    }

    /// The recovery capture reads top-to-bottom: title (header) → explanation → the code → the
    /// confirm button. The code is reachable and announced (a value to note).
    func testRecoveryCaptureReadingOrder() {
        let model = DriverOnboardingScreens.recoveryCodeCapture(code: Fixtures.recoveryCode, onContinue: {})
        let order = model.a11yReadingOrder
        XCTAssertEqual(order.first?.label, DriverL10n.recoveryTitle)
        XCTAssertEqual(order.first?.isHeader, true)
        XCTAssertEqual(order.last?.label, DriverL10n.recoverySaved)
        XCTAssertEqual(order.last?.isButton, true)
        XCTAssertTrue(order.contains { $0.label == Fixtures.recoveryCode && $0.isStaticText })
    }

    func testAutoUpdateEnabledIsAnnouncedAsStateNotButton() {
        let model = RiderOnboardingScreens.autoUpdateEnabled(onContinue: {})
        let confirmation = model.a11yReadingOrder.first { $0.label == L10n.autoUpdateEnabled }
        XCTAssertNotNil(confirmation)
        XCTAssertEqual(confirmation?.isStaticText, true)
        XCTAssertEqual(confirmation?.isButton, false, "'auto-update enabled' must not be a button (a11y).")
    }
}
