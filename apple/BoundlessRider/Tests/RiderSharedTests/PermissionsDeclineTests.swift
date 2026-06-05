import BoundlessKit
import XCTest

@testable import RiderShared

/// AC14 (UI leg) — on a declined (or OS-denied) notification permission, onboarding still advances
/// to the auto-update step AND the model marks the non-PII admin flag. It never blocks or scolds.
/// The "server receives the flag" integration leg is the server task T07.
@MainActor
final class PermissionsDeclineTests: XCTestCase {
    func testNotNowAdvancesAndFlags() async {
        let vm = makeOnboardingVM(granted: false)
        await advanceToPermissions(vm)
        XCTAssertEqual(vm.state, .permissions)

        await vm.decideNotifications(allow: false)

        XCTAssertEqual(vm.state, .autoUpdateStep, "decline must still advance the flow (AC14).")
        XCTAssertTrue(vm.notificationsFlaggedOff, "decline must set the non-PII admin flag (AC14).")
    }

    func testGrantAdvancesWithoutFlag() async {
        let vm = makeOnboardingVM(granted: true)
        await advanceToPermissions(vm)

        await vm.decideNotifications(allow: true)

        XCTAssertEqual(vm.state, .autoUpdateStep)
        XCTAssertFalse(vm.notificationsFlaggedOff)
    }

    func testAllowButOSDeniedStillFlagsAndAdvances() async {
        // The helper taps "Turn on notifications" but the OS prompt is denied → still flag, still advance.
        let vm = makeOnboardingVM(granted: false)
        await advanceToPermissions(vm)

        await vm.decideNotifications(allow: true)

        XCTAssertEqual(vm.state, .autoUpdateStep)
        XCTAssertTrue(vm.notificationsFlaggedOff)
    }
}
