import BoundlessKit
import XCTest

@testable import RiderShared

/// Drives the onboarding view model through the state machine, asserting it RENDERS the core's
/// transitions and never decides them itself (P4). Covers the happy path, the recovery branches,
/// the cross-cutting events, launch routing, and the offline-overlay gating the UI relies on.
@MainActor
final class OnboardingViewModelTests: XCTestCase {
    func testHappyPathReachesSilentComplete() async {
        let vm = makeOnboardingVM(granted: true)
        XCTAssertEqual(vm.state, .freshInstall)
        vm.begin()
        XCTAssertEqual(vm.state, .phoneEntry)
        await vm.submitPhone("5551234567")
        XCTAssertEqual(vm.state, .deviceBinding)
        await vm.submitCode("123456")
        XCTAssertEqual(vm.state, .permissions)
        await vm.decideNotifications(allow: true)
        XCTAssertEqual(vm.state, .autoUpdateStep)
        vm.confirmAutoUpdate()
        XCTAssertEqual(vm.state, .complete)
        XCTAssertTrue(isTerminal(state: vm.state))
        XCTAssertFalse(vm.notificationsFlaggedOff)
    }

    func testPhoneNotOnFileReturnsThenRecovers() async {
        let net = StubNetworking(signIn: .phoneNotOnFile)
        let vm = OnboardingViewModel(
            role: .rider, hasValidSession: false, networking: net,
            notifications: StubNotifications(granted: true), manifest: StubManifest(adminName: "Sarah")
        )
        vm.begin()
        await vm.submitPhone("000")
        XCTAssertEqual(vm.state, .phoneNotOnFile)

        net.signInResult = .memberMatched
        await vm.submitPhone("5551234567")
        XCTAssertEqual(vm.state, .deviceBinding)
    }

    func testBindingFailedReturnsThenRecovers() async {
        let net = StubNetworking(signIn: .memberMatched, bind: .failed)
        let vm = OnboardingViewModel(
            role: .rider, hasValidSession: false, networking: net,
            notifications: StubNotifications(granted: true), manifest: StubManifest(adminName: "Sarah")
        )
        vm.begin()
        await vm.submitPhone("5551234567")
        await vm.submitCode("000000")
        XCTAssertEqual(vm.state, .bindingFailed)

        net.bindResult = .bound
        await vm.submitCode("123456")
        XCTAssertEqual(vm.state, .permissions)
    }

    func testBelowMinVersionReachableMidFlow() async {
        let vm = makeOnboardingVM()
        vm.begin()
        vm.belowMinVersionDetected()
        XCTAssertEqual(vm.state, .belowMinVersion)
    }

    func testSessionInvalidatedRoutesByRole() async {
        let rider = makeOnboardingVM(role: .rider)
        await advanceToPermissions(rider)
        rider.sessionInvalidated()
        XCTAssertEqual(rider.state, .needsReauthHelp)

        let driver = makeOnboardingVM(role: .driver)
        await advanceToPermissions(driver)
        driver.sessionInvalidated()
        XCTAssertEqual(driver.state, .phoneEntry)
    }

    func testLaunchRoutingResumesLiveSession() {
        XCTAssertEqual(makeOnboardingVM(hasValidSession: true).state, .complete)
        XCTAssertEqual(makeOnboardingVM(hasValidSession: false).state, .freshInstall)
    }

    func testOfflineOverlayGatingMatchesCore() {
        // The router only offers the Offline overlay where the core allows it (sign-in / binding).
        XCTAssertTrue(allowsOfflineOverlay(state: .phoneEntry))
        XCTAssertTrue(allowsOfflineOverlay(state: .deviceBinding))
        XCTAssertFalse(allowsOfflineOverlay(state: .permissions))
        XCTAssertFalse(allowsOfflineOverlay(state: .belowMinVersion))
        XCTAssertFalse(allowsOfflineOverlay(state: .needsReauthHelp))
    }
}
