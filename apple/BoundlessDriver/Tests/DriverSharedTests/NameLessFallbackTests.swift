import RiderShared
import SwiftUI
import XCTest

@testable import DriverShared

/// The Driver reuses the name-bearing shared screens (phone-not-on-file, device binding, binding
/// failed, permissions declined, the calm degradation screen). When no manifest/admin name is cached
/// they must render the name-less fallback — never an empty `%1$@` slot — so no dangling punctuation
/// reaches the Driver (P10 / voice-and-tone). RiderShared owns the fallback logic (tested in T11);
/// this re-asserts it for the Driver's screen set so a future change can't silently break it here.
final class NameLessFallbackTests: XCTestCase {
    func testNoDriverScreenRendersDanglingPunctuationWhenNameMissing() {
        let nameless: [OnboardingScreenModel] = [
            RiderOnboardingScreens.phoneNotOnFile(text: .constant(""), adminName: nil, onTryAgain: {}),
            RiderOnboardingScreens.deviceBinding(text: .constant(""), adminName: nil, onContinue: {}),
            RiderOnboardingScreens.bindingFailed(text: .constant(""), adminName: nil, onTryAgain: {}),
            RiderOnboardingScreens.permissionsDeclined(adminName: nil, onContinue: {}),
            RiderOnboardingScreens.calmHelp(adminName: nil),
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
