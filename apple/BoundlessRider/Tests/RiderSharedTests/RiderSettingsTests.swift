import XCTest

@testable import RiderShared

/// AC6 — the Rider's Settings UI does not surface an "automatic updates" toggle (asserts O3). The
/// absence is structural: `RiderSettingsRow` has no such case, so a row for it cannot be built.
final class RiderSettingsTests: XCTestCase {
    func testNoAutomaticUpdatesToggle() {
        XCTAssertFalse(RiderSettingsModel().surfacesAutomaticUpdatesToggle)
    }

    func testNoSettingsRowMentionsAutomaticUpdates() {
        let forbidden = ["update", "automatic", "auto-update"]
        for row in RiderSettingsRow.allCases {
            let title = row.title.lowercased()
            for term in forbidden {
                XCTAssertFalse(
                    title.contains(term),
                    "Rider settings row '\(row.title)' must not mention app updates (AC6)."
                )
            }
        }
    }

    func testReadingOrderHasNoUpdateAffordance() {
        for descriptor in RiderSettingsModel().a11yReadingOrder {
            XCTAssertFalse(descriptor.label.lowercased().contains("update"))
        }
    }
}
