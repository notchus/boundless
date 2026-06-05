import BoundlessKit
import XCTest

@testable import RiderShared

/// AC15 / P10 — a Rider whose session expired sees the calm `NeedsReauthHelp` screen: no
/// phone-entry field, no sign-in form. (The "one alert per member per day" integration leg is the
/// server task T07; the snapshot leg is in `OnboardingSnapshotTests`.)
final class NeedsReauthHelpTests: XCTestCase {
    func testCalmScreenHasNoFormOrCTA() {
        for adminName in [Fixtures.adminName, nil] {
            let model = RiderOnboardingScreens.calmHelp(adminName: adminName)
            XCTAssertFalse(model.hasInputAffordance, "NeedsReauthHelp must show no field (AC15).")
            XCTAssertTrue(model.actions.isEmpty, "NeedsReauthHelp must show no sign-in / CTA (AC15).")
        }
    }

    func testLoneRiderRoutesToCalmScreenNotAForm() {
        // The core routes a lone Rider to the calm help screen (and a Driver to interactive re-auth).
        XCTAssertEqual(reauthStateFor(role: .rider), .needsReauthHelp)
        XCTAssertEqual(reauthStateFor(role: .driver), .phoneEntry)
    }
}
