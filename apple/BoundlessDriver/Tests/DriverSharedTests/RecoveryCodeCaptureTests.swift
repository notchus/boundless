import BoundlessKit
import RiderShared
import SwiftUI
import XCTest

@testable import DriverShared

/// AC19 (UI capture leg) — the Driver's one-time **Recovery Code capture** screen (ADR-0016 D3).
/// The screen shows the server-minted code with save instructions so the Driver can self-serve a
/// future device replacement. Riders have no such screen (recovery is Admin-mediated). The capture
/// is shown once, right after the device is bound (the core's `Permissions` state).
final class RecoveryCodeCaptureTests: XCTestCase {
    private let code = Fixtures.recoveryCode

    /// The capture screen renders the code (as data, via the `.code` element), the title, the
    /// explanation and the "I've saved it" confirm action — and is **not** a sign-in form.
    func testCaptureScreenContent() {
        let model = DriverOnboardingScreens.recoveryCodeCapture(code: code, onContinue: {})

        XCTAssertTrue(model.elements.contains(.heading(DriverL10n.recoveryTitle)))
        XCTAssertTrue(model.elements.contains(.paragraph(DriverL10n.recoveryExplanation)))
        XCTAssertTrue(model.elements.contains(.code(code)), "The Recovery Code must be displayed.")
        XCTAssertEqual(model.actionLabels, [DriverL10n.recoverySaved])
        XCTAssertFalse(model.hasInputAffordance, "Capture screen is not a form — it displays a code.")
    }

    /// VoiceOver must read the code: it appears in the reading order as static text (a value to note,
    /// not a control).
    func testCodeIsInReadingOrderAsStaticText() {
        let model = DriverOnboardingScreens.recoveryCodeCapture(code: code, onContinue: {})
        let descriptor = model.a11yReadingOrder.first { $0.label == code }
        XCTAssertNotNil(descriptor, "The code must be in the VoiceOver reading order.")
        XCTAssertEqual(descriptor?.isStaticText, true)
        XCTAssertEqual(descriptor?.isButton, false, "The code is a value to read, not a button.")
    }

    /// After a successful bind the core is at `Permissions` and a Recovery Code is available — the
    /// router's precondition for showing the capture interstitial (`!captured && code != nil`).
    @MainActor
    func testCodeIsAvailableAtPermissionsAfterBind() async {
        let vm = makeDriverVM()
        await advanceToPermissions(vm)
        XCTAssertEqual(vm.state, .permissions)
        XCTAssertEqual(vm.recoveryCode, code)
    }

    /// Degenerate shell case: no code captured → the router skips the capture (never an empty-code
    /// screen, never a block). Modelled by `recoveryCode == nil`.
    @MainActor
    func testNoCodeSkipsCapture() async {
        let vm = makeDriverVM(recoveryCode: nil)
        await advanceToPermissions(vm)
        XCTAssertEqual(vm.state, .permissions)
        XCTAssertNil(vm.recoveryCode, "With no code the router proceeds straight to permissions.")
    }

    /// AC19 contrast: the capture is **Driver-only**. The Driver screen set includes it; the shared
    /// `RiderOnboardingScreens` kit has no such factory (compile-time fact — Riders recover via Admin).
    @MainActor
    func testCaptureIsInTheDriverScreenSet() {
        let names = DriverScreenFixtures.allModels().map(\.name)
        XCTAssertTrue(names.contains("recoveryCodeCapture"))
    }
}
