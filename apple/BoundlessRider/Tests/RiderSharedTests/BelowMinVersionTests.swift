import BoundlessKit
import XCTest

@testable import RiderShared

/// AC8 (snapshot/UI leg) — a below-`client_min_version` client sees only the calm degradation
/// screen, with no "Update Now" control (asserts O4/O8). The "one alert per member per day"
/// integration leg is the server task T07; the snapshot is in `OnboardingSnapshotTests`.
final class BelowMinVersionTests: XCTestCase {
    func testNamedScreenRepeatsTheAdminName() {
        let model = RiderOnboardingScreens.calmHelp(adminName: "Sarah")
        guard case let .heading(text)? = model.elements.first else {
            return XCTFail("calm screen should lead with the message")
        }
        XCTAssertEqual(text, "This device needs Sarah's help. Sarah has been told.")
        // Repeat the name, never a pronoun (translates correctly across gendered languages).
        XCTAssertEqual(text.components(separatedBy: "Sarah").count - 1, 2)
    }

    func testNameLessFallbackWhenNoManifest() {
        let model = RiderOnboardingScreens.calmHelp(adminName: nil)
        XCTAssertTrue(model.elements.contains(.heading(L10n.belowMinVersionGeneric)))
    }

    func testNoUpdateNowControl() {
        for adminName in ["Sarah", nil] {
            let model = RiderOnboardingScreens.calmHelp(adminName: adminName)
            XCTAssertTrue(model.actions.isEmpty, "below-min screen must have no CTA (no 'Update Now', O8).")
            for label in model.actionLabels {
                XCTAssertFalse(label.lowercased().contains("update"))
            }
        }
    }

    func testReachableFromAnyState() {
        // O4: the degradation is reachable from any auth response / handshake, not only sign-in.
        XCTAssertEqual(onEvent(state: .phoneEntry, event: .belowMinVersionDetected), .belowMinVersion)
        XCTAssertEqual(onEvent(state: .permissions, event: .belowMinVersionDetected), .belowMinVersion)
    }
}
