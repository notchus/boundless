package app.boundless.driver

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.boundless_ffi_kotlin.OnboardingState
import uniffi.boundless_ffi_kotlin.isTerminal

/**
 * The Driver flow drives the same `core::auth` state machine as the Rider (composed with
 * `role = DRIVER`) — every transition is the core's (P4). These assert the Driver happy path, the
 * AC14 decline-and-flag branch, the O4 below-min override, and the resume short-circuit. The Android
 * twin of `DriverOnboardingViewModelTests.swift`. Every transition goes through `:core-bridge`.
 */
class DriverOnboardingViewModelTest {
    @Test
    fun happyPathReachesSilentComplete() = runTest {
        val vm = makeDriverVM(granted = true)
        assertEquals(OnboardingState.FRESH_INSTALL, vm.state)
        vm.begin()
        assertEquals(OnboardingState.PHONE_ENTRY, vm.state)
        vm.submitPhone("5551234567")
        assertEquals(OnboardingState.DEVICE_BINDING, vm.state)
        vm.submitCode("123456")
        assertEquals(OnboardingState.PERMISSIONS, vm.state)
        vm.decideNotifications(allow = true)
        assertEquals(OnboardingState.AUTO_UPDATE_STEP, vm.state)
        vm.confirmAutoUpdate()
        assertEquals(OnboardingState.COMPLETE, vm.state)
        assertTrue(isTerminal(vm.state))
        assertFalse(vm.notificationsFlaggedOff)
    }

    /** AC14 — a declined permission still advances the flow AND records the non-PII flag (via the
     *  core's `shouldFlagNotificationsOff`). Never blocks, never scolds. */
    @Test
    fun declinedPermissionAdvancesAndFlags() = runTest {
        val vm = makeDriverVM()
        advanceToPermissions(vm)
        vm.decideNotifications(allow = false)
        assertEquals(OnboardingState.AUTO_UPDATE_STEP, vm.state)
        assertTrue("Decline must set the non-PII admin flag (AC14).", vm.notificationsFlaggedOff)
    }

    /** O4 — a below-`client_min_version` handshake overrides from any state to the calm screen. */
    @Test
    fun belowMinVersionOverridesFromAnyState() = runTest {
        val vm = makeDriverVM()
        vm.begin()
        vm.submitPhone("5551234567")
        assertEquals(OnboardingState.DEVICE_BINDING, vm.state)
        vm.belowMinVersionDetected()
        assertEquals(OnboardingState.BELOW_MIN_VERSION, vm.state)
    }

    /** A live session resumes straight to the primary surface (no re-onboarding) — the launch
     *  decision is the core's (`launch(hasValidSession)`). */
    @Test
    fun liveSessionResumesToComplete() {
        val vm = makeDriverVM(hasValidSession = true)
        assertEquals(OnboardingState.COMPLETE, vm.state)
        assertFalse(vm.reauthRequested)
    }
}
