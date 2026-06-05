import BoundlessKit
import RiderShared
import SwiftUI
import XCTest

@testable import DriverShared

/// AC15 (Driver branch) — a Driver whose session expired is routed to **interactive re-auth** at
/// `PhoneEntry` (showing `auth.signin_again`), unlike a Rider, who sees the form-less calm
/// `NeedsReauthHelp`. The divergence is the core's decision (`reauth_state_for`), not the UI's (P4).
final class DriverReauthRouteTests: XCTestCase {
    /// The core routes Driver vs Rider differently — this is the whole point of the branch.
    func testCoreRoutesDriverToPhoneEntryAndRiderToHelp() {
        XCTAssertEqual(reauthStateFor(role: .driver), .phoneEntry)
        XCTAssertEqual(reauthStateFor(role: .rider), .needsReauthHelp)
    }

    /// An invalidated Driver session lands the VM at `PhoneEntry` and records `reauthRequested`, so
    /// the router shows the re-auth variant rather than the fresh sign-in. The target state is the
    /// core's (`reauth_state_for(.driver)`); the flag only records that it happened.
    @MainActor
    func testSessionInvalidatedRoutesToPhoneEntryReauth() {
        let vm = makeDriverVM()
        vm.begin()  // FreshInstall → PhoneEntry, then drive into the flow before invalidation
        XCTAssertFalse(vm.reauthRequested)

        vm.sessionInvalidated()
        XCTAssertEqual(vm.state, .phoneEntry)
        XCTAssertTrue(vm.reauthRequested)
    }

    /// The re-auth screen leads with `auth.signin_again` and **is** a sign-in form (a Driver
    /// self-re-auths) — the deliberate contrast with the Rider's form-less calm screen.
    func testReAuthScreenIsASignInFormLedBySignInAgain() {
        let model = DriverOnboardingScreens.reAuthPhoneEntry(text: .constant(""), onContinue: {})

        XCTAssertEqual(model.a11yReadingOrder.first?.label, L10n.signInAgain)
        XCTAssertEqual(model.a11yReadingOrder.first?.isHeader, true, "Re-auth leads with a header.")
        XCTAssertTrue(model.hasInputAffordance, "A Driver re-auths interactively — this IS a form.")

        // Contrast: a Rider in the same situation gets the form-less calm screen.
        XCTAssertFalse(RiderOnboardingScreens.calmHelp(adminName: Fixtures.adminName).hasInputAffordance)
    }
}
