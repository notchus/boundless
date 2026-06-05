import BoundlessKit
import RiderShared
import XCTest

@testable import DriverShared

/// AC1(b) — the **Driver** onboarding entry flow exposes no "sign up" / "create account" / "request
/// access" route (asserts I11). Verified against the state machine (which has no signup state) and
/// against every rendered Driver screen's copy + affordances.
final class NoDriverSignupRouteTests: XCTestCase {
    private let forbidden = [
        "sign up", "signup", "sign-up",
        "create account", "create an account", "create your account",
        "request access", "request an account",
        "register", "join now", "get started",
    ]

    func test_ios_driver_no_signup_route() {
        for (name, model) in DriverScreenFixtures.allModels() {
            let strings = textContent(of: model) + model.actionLabels
            for text in strings {
                let lower = text.lowercased()
                for term in forbidden {
                    XCTAssertFalse(
                        lower.contains(term),
                        "Driver screen '\(name)' exposes a signup-like affordance/copy: '\(text)' (AC1(b))."
                    )
                }
            }
        }
    }

    /// The entry from a fresh install is sign-IN (to an existing admin-issued account), never
    /// sign-UP: the only forward transition from `FreshInstall` leads to `PhoneEntry` (I11). Even the
    /// Driver's interactive re-auth lands on `PhoneEntry`, never an account-creation screen.
    func testEntryAndReauthAreSignInNotSignUp() {
        XCTAssertEqual(onEvent(state: .freshInstall, event: .beginSignIn), .phoneEntry)
        XCTAssertEqual(reauthStateFor(role: .driver), .phoneEntry)
    }

    private func textContent(of model: OnboardingScreenModel) -> [String] {
        model.elements.map { element in
            switch element {
            case let .heading(t), let .paragraph(t), let .banner(t), let .confirmation(t), let .code(t):
                return t
            }
        }
    }
}
