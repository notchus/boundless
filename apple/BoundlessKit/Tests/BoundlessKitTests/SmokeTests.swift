import XCTest

@testable import BoundlessKit

/// End-to-end proof that the Rust `core::auth` onboarding state machine crosses the
/// Rust → UniFFI → Swift boundary and executes correctly **on the iOS simulator** (spec 001
/// T10-shell). This is BoundlessKit's reason to exist: the Swift clients (T11+) render these
/// states; they never re-implement the rules (P4). It exercises plain enums, a data-carrying
/// enum (`OnboardingEvent`), and free functions — the full shape T11 consumes.
final class BoundlessKitSmokeTests: XCTestCase {
    func testLaunchRoutingCrossesTheFFI() {
        XCTAssertEqual(launch(hasValidSession: false), .onboard)
        XCTAssertEqual(launch(hasValidSession: true), .resume)
    }

    func testHappyPathTraversal() {
        var s: OnboardingState = .freshInstall
        s = onEvent(state: s, event: .beginSignIn)
        XCTAssertEqual(s, .phoneEntry)
        s = onEvent(state: s, event: .signIn(result: .memberMatched))
        XCTAssertEqual(s, .deviceBinding)
        s = onEvent(state: s, event: .bind(result: .bound))
        XCTAssertEqual(s, .permissions)
        s = onEvent(state: s, event: .permissionDecision(granted: false))
        XCTAssertEqual(s, .autoUpdateStep)
        s = onEvent(state: s, event: .autoUpdateConfirmed)
        XCTAssertEqual(s, .complete)
        XCTAssertTrue(isTerminal(state: s))
    }

    func testReauthRoutingByRole() {
        // A lone Rider sees the calm help screen, never a sign-in form (AC15/P10).
        XCTAssertEqual(reauthStateFor(role: .rider), .needsReauthHelp)
        // A Driver routes to interactive re-auth.
        XCTAssertEqual(reauthStateFor(role: .driver), .phoneEntry)
    }

    func testCrossCuttingOverridesAndOverlay() {
        // Below-min and session-invalidated arrive from any state (O4 / AC15).
        XCTAssertEqual(
            onEvent(state: .permissions, event: .belowMinVersionDetected),
            .belowMinVersion
        )
        XCTAssertEqual(
            onEvent(state: .deviceBinding, event: .sessionInvalidated(role: .rider)),
            .needsReauthHelp
        )
        // The Offline overlay is only allowed over the sign-in/binding steps.
        XCTAssertTrue(allowsOfflineOverlay(state: .phoneEntry))
        XCTAssertTrue(allowsOfflineOverlay(state: .deviceBinding))
        XCTAssertFalse(allowsOfflineOverlay(state: .permissions))
    }

    func testNotificationDeclineFlag() {
        XCTAssertTrue(shouldFlagNotificationsOff(granted: false))
        XCTAssertFalse(shouldFlagNotificationsOff(granted: true))
    }
}
