import BoundlessKit
import XCTest

@testable import DriverShared

/// The Driver flow drives the same `core::auth` state machine as the Rider (composed with
/// `role: .driver`) — every transition is the core's (P4). These assert the Driver happy path,
/// the AC14 decline-and-flag branch, the O4 below-min override, and the resume short-circuit.
@MainActor
final class DriverOnboardingViewModelTests: XCTestCase {
    func testHappyPathReachesComplete() async {
        let vm = makeDriverVM(granted: true)
        XCTAssertEqual(vm.state, .freshInstall)

        vm.begin()
        XCTAssertEqual(vm.state, .phoneEntry)

        await vm.submitPhone("5551234567")
        XCTAssertEqual(vm.state, .deviceBinding)

        await vm.submitCode("123456")
        XCTAssertEqual(vm.state, .permissions)

        await vm.decideNotifications(allow: true)
        XCTAssertEqual(vm.state, .autoUpdateStep)
        XCTAssertFalse(vm.notificationsFlaggedOff)

        vm.confirmAutoUpdate()
        XCTAssertEqual(vm.state, .complete)
    }

    /// AC14 — a declined permission still advances the flow AND records the non-PII flag (via the
    /// core's `should_flag_notifications_off`). Never blocks, never scolds.
    func testDeclinedPermissionAdvancesAndFlags() async {
        let vm = makeDriverVM()
        await advanceToPermissions(vm)

        await vm.decideNotifications(allow: false)
        XCTAssertEqual(vm.state, .autoUpdateStep)
        XCTAssertTrue(vm.notificationsFlaggedOff, "Decline must set the non-PII admin flag (AC14).")
    }

    /// O4 — a below-`client_min_version` handshake overrides from any state to the calm screen.
    func testBelowMinVersionOverridesFromAnyState() async {
        let vm = makeDriverVM()
        vm.begin()
        await vm.submitPhone("5551234567")
        XCTAssertEqual(vm.state, .deviceBinding)

        vm.belowMinVersionDetected()
        XCTAssertEqual(vm.state, .belowMinVersion)
    }

    /// A live session resumes straight to the primary surface (no re-onboarding) — the launch
    /// decision is the core's (`launch(hasValidSession:)`).
    func testLiveSessionResumesToComplete() {
        let vm = makeDriverVM(hasValidSession: true)
        XCTAssertEqual(vm.state, .complete)
        XCTAssertFalse(vm.reauthRequested)
    }
}
