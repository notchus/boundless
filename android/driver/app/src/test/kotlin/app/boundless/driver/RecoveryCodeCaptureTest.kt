package app.boundless.driver

import app.boundless.rider.onboarding.BodyElement
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.boundless_ffi_kotlin.OnboardingState

/**
 * AC19 (UI capture leg) — the Driver's one-time **Recovery Code capture** screen (ADR-0016 D3). The
 * screen shows the server-minted code with save instructions so the Driver can self-serve a future
 * device replacement. Riders have no such screen (recovery is Admin-mediated). The capture is shown
 * once, right after the device is bound (the core's `Permissions` state). Twin of
 * `RecoveryCodeCaptureTests.swift`.
 */
class RecoveryCodeCaptureTest {
    private val code = Fixtures.RECOVERY_CODE

    /** The capture screen renders the code (as data, via the `Code` element), the title, the
     *  explanation and the "I've saved it" confirm action — and is **not** a sign-in form. */
    @Test
    fun captureScreenContent() {
        val model = driverScreens().recoveryCodeCapture(code) {}

        assertTrue(model.elements.contains(BodyElement.Heading(TestStrings.recoveryTitle)))
        assertTrue(model.elements.contains(BodyElement.Paragraph(TestStrings.recoveryExplanation)))
        assertTrue("The Recovery Code must be displayed.", model.elements.contains(BodyElement.Code(code)))
        assertEquals(listOf(TestStrings.recoverySaved), model.actionLabels)
        assertFalse("Capture screen is not a form — it displays a code.", model.hasInputAffordance)
    }

    /** TalkBack must read the code: it appears in the reading order as static text (a value to note,
     *  not a control). */
    @Test
    fun codeIsInReadingOrderAsStaticText() {
        val model = driverScreens().recoveryCodeCapture(code) {}
        val descriptor = model.a11yReadingOrder.firstOrNull { it.label == code }
        assertTrue("The code must be in the TalkBack reading order.", descriptor != null)
        assertEquals(true, descriptor?.isStaticText)
        assertEquals("The code is a value to read, not a button.", false, descriptor?.isButton)
    }

    /** After a successful bind the core is at `Permissions` and a Recovery Code is available — the
     *  router's precondition for showing the capture interstitial (`!captured && code != null`). */
    @Test
    fun codeIsAvailableAtPermissionsAfterBind() = runTest {
        val vm = makeDriverVM()
        advanceToPermissions(vm)
        assertEquals(OnboardingState.PERMISSIONS, vm.state)
        assertEquals(code, vm.recoveryCode)
    }

    /** Degenerate shell case: no code captured → the router skips the capture (never an empty-code
     *  screen, never a block). Modelled by `recoveryCode == null`. */
    @Test
    fun noCodeSkipsCapture() = runTest {
        val vm = makeDriverVM(recoveryCode = null)
        advanceToPermissions(vm)
        assertEquals(OnboardingState.PERMISSIONS, vm.state)
        assertNull("With no code the router proceeds straight to permissions.", vm.recoveryCode)
    }

    /** AC19 contrast: the capture is **Driver-only**. The Driver screen set includes it; the shared
     *  `RiderOnboardingScreens` kit has no such factory (compile-time fact — Riders recover via Admin). */
    @Test
    fun captureIsInTheDriverScreenSet() {
        assertTrue(DriverScreenFixtures.allModels().map { it.first }.contains("recoveryCodeCapture"))
    }
}
