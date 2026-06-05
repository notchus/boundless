import SwiftUI
import XCTest

@testable import RiderShared

/// Reviewer T11 (confirmed, medium): the four name-bearing screens must NOT render a broken sentence
/// when no manifest/admin name is cached (first-launch manifest race / verify failure). With
/// `adminName == nil` they resolve a name-less fallback (mirroring `auth.below_min_version_generic`),
/// never an empty `%1$@` slot — so no dangling punctuation reaches an already-confused rider (P10 /
/// voice-and-tone). Guards against a regression to the old `adminName ?? ""` coercion.
final class NameLessFallbackTests: XCTestCase {
    func testNameLessAccessorsUseTheGenericFallback() {
        XCTAssertEqual(
            L10n.phoneNotOnFile(adminName: nil),
            "That number doesn't match what's on file. Try again, or your group can help."
        )
        XCTAssertEqual(L10n.codePrompt(adminName: nil), "Enter your Onboarding Code.")
        XCTAssertEqual(
            L10n.codeInvalid(adminName: nil),
            "That code didn't work. Your group can give you a new one."
        )
        XCTAssertEqual(
            L10n.notificationsDeclined(adminName: nil),
            "We'll let your group know notifications aren't on yet."
        )
    }

    func testNameLessAccessorsStillSubstituteWhenNamed() {
        XCTAssertEqual(L10n.codePrompt(adminName: "Sarah"), "Enter the Onboarding Code from Sarah.")
        XCTAssertTrue(L10n.codeInvalid(adminName: "Sarah").contains("Sarah"))
    }

    /// No name-bearing screen, with adminName nil, may render a broken sentence: no leftover `%`
    /// format specifier, no empty-slot artifacts (" ." / " ," / "  ").
    func testNoScreenRendersDanglingPunctuationWhenNameMissing() {
        let nameless: [OnboardingScreenModel] = [
            RiderOnboardingScreens.phoneNotOnFile(text: .constant(""), adminName: nil, onTryAgain: {}),
            RiderOnboardingScreens.deviceBinding(text: .constant(""), adminName: nil, onContinue: {}),
            RiderOnboardingScreens.bindingFailed(text: .constant(""), adminName: nil, onTryAgain: {}),
            RiderOnboardingScreens.permissionsDeclined(adminName: nil, onContinue: {}),
        ]
        for model in nameless {
            for descriptor in model.a11yReadingOrder {
                let text = descriptor.label
                XCTAssertFalse(text.contains("%"), "Unsubstituted format specifier in: '\(text)'")
                XCTAssertFalse(text.contains(" ."), "Dangling space-period in: '\(text)'")
                XCTAssertFalse(text.contains(" ,"), "Dangling space-comma in: '\(text)'")
                XCTAssertFalse(text.contains("  "), "Double space (empty slot) in: '\(text)'")
            }
        }
    }
}
