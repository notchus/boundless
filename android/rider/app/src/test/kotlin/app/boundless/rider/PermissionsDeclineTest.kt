package app.boundless.rider

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.boundless_ffi_kotlin.OnboardingState

/**
 * AC14 (UI leg) — on a declined (or OS-denied) notification permission, onboarding still advances to
 * the auto-update step AND the model marks the non-PII admin flag. It never blocks or scolds. The
 * "server receives the flag" integration leg is the server task T07. Twin of `PermissionsDeclineTests.swift`.
 */
class PermissionsDeclineTest {
    @Test
    fun notNowAdvancesAndFlags() = runTest {
        val vm = makeOnboardingVM(granted = false)
        advanceToPermissions(vm)
        assertEquals(OnboardingState.PERMISSIONS, vm.state)

        vm.decideNotifications(allow = false)

        assertEquals("decline must still advance the flow (AC14).", OnboardingState.AUTO_UPDATE_STEP, vm.state)
        assertTrue("decline must set the non-PII admin flag (AC14).", vm.notificationsFlaggedOff)
    }

    @Test
    fun grantAdvancesWithoutFlag() = runTest {
        val vm = makeOnboardingVM(granted = true)
        advanceToPermissions(vm)

        vm.decideNotifications(allow = true)

        assertEquals(OnboardingState.AUTO_UPDATE_STEP, vm.state)
        assertFalse(vm.notificationsFlaggedOff)
    }

    @Test
    fun allowButOSDeniedStillFlagsAndAdvances() = runTest {
        // The helper taps "Turn on notifications" but the OS prompt is denied → still flag, still advance.
        val vm = makeOnboardingVM(granted = false)
        advanceToPermissions(vm)

        vm.decideNotifications(allow = true)

        assertEquals(OnboardingState.AUTO_UPDATE_STEP, vm.state)
        assertTrue(vm.notificationsFlaggedOff)
    }
}
