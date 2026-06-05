import BoundlessKit
import XCTest

@testable import RiderShared

/// AC1(b) — the Rider onboarding entry flow exposes no "sign up" / "create account" / "request
/// access" route (asserts I11). Verified against the state machine (which has no signup state) and
/// against every rendered screen's copy + affordances.
final class NoSignupRouteTests: XCTestCase {
    private let forbidden = [
        "sign up", "signup", "sign-up",
        "create account", "create an account", "create your account",
        "request access", "request an account",
        "register", "join now", "get started",
    ]

    func test_ios_onboarding_no_signup_route() {
        for (name, model) in ScreenFixtures.allModels() {
            let strings = textContent(of: model) + model.actionLabels
            for text in strings {
                let lower = text.lowercased()
                for term in forbidden {
                    XCTAssertFalse(
                        lower.contains(term),
                        "Screen '\(name)' exposes a signup-like affordance/copy: '\(text)' (AC1(b))."
                    )
                }
            }
        }
    }

    /// The entry from a fresh install is sign-IN (to an existing admin-issued account), never
    /// sign-UP: the only forward transition from `FreshInstall` leads to `PhoneEntry` (I11).
    func testEntryFromFreshInstallIsSignIn() {
        XCTAssertEqual(onEvent(state: .freshInstall, event: .beginSignIn), .phoneEntry)
    }

    /// A lone Rider with an invalidated session cannot self-serve into account creation — the calm
    /// terminal screen has no input at all (reinforces AC1(b) alongside AC15).
    func testTerminalScreensHaveNoInput() {
        XCTAssertFalse(RiderOnboardingScreens.calmHelp(adminName: Fixtures.adminName).hasInputAffordance)
        XCTAssertFalse(RiderOnboardingScreens.calmHelp(adminName: nil).hasInputAffordance)
    }

    private func textContent(of model: OnboardingScreenModel) -> [String] {
        model.elements.map { element in
            switch element {
            case let .heading(t), let .paragraph(t), let .banner(t), let .confirmation(t): return t
            }
        }
    }
}
