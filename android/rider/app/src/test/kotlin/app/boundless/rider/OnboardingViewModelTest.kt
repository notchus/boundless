package app.boundless.rider

import app.boundless.rider.onboarding.OnboardingViewModel
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.boundless_ffi_kotlin.OnboardingState
import uniffi.boundless_ffi_kotlin.Role
import uniffi.boundless_ffi_kotlin.SignInResult
import uniffi.boundless_ffi_kotlin.allowsOfflineOverlay
import uniffi.boundless_ffi_kotlin.isTerminal

/**
 * Drives the onboarding view model through the state machine, asserting it RENDERS the core's
 * transitions and never decides them itself (P4). Covers the happy path, the recovery branches, the
 * cross-cutting events, launch routing, and the offline-overlay gating the UI relies on. The Android
 * twin of `OnboardingViewModelTests.swift`. Every transition goes through `:core-bridge`.
 */
class OnboardingViewModelTest {
    @Test
    fun happyPathReachesSilentComplete() = runTest {
        val vm = makeOnboardingVM(granted = true)
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

    @Test
    fun phoneNotOnFileReturnsThenRecovers() = runTest {
        val net = FakeNetworking(signInResult = SignInResult.PHONE_NOT_ON_FILE)
        val vm = OnboardingViewModel(Role.RIDER, false, net, FakeNotifications(true), FakeManifest("Sarah"))
        vm.begin()
        vm.submitPhone("000")
        assertEquals(OnboardingState.PHONE_NOT_ON_FILE, vm.state)

        net.signInResult = SignInResult.MEMBER_MATCHED
        vm.submitPhone("5551234567")
        assertEquals(OnboardingState.DEVICE_BINDING, vm.state)
    }

    @Test
    fun bindingFailedReturnsThenRecovers() = runTest {
        val net = FakeNetworking(signInResult = SignInResult.MEMBER_MATCHED, bindResult = uniffi.boundless_ffi_kotlin.BindResult.FAILED)
        val vm = OnboardingViewModel(Role.RIDER, false, net, FakeNotifications(true), FakeManifest("Sarah"))
        vm.begin()
        vm.submitPhone("5551234567")
        vm.submitCode("000000")
        assertEquals(OnboardingState.BINDING_FAILED, vm.state)

        net.bindResult = uniffi.boundless_ffi_kotlin.BindResult.BOUND
        vm.submitCode("123456")
        assertEquals(OnboardingState.PERMISSIONS, vm.state)
    }

    @Test
    fun belowMinVersionReachableMidFlow() {
        val vm = makeOnboardingVM()
        vm.begin()
        vm.belowMinVersionDetected()
        assertEquals(OnboardingState.BELOW_MIN_VERSION, vm.state)
    }

    @Test
    fun sessionInvalidatedRoutesByRole() = runTest {
        val rider = makeOnboardingVM(role = Role.RIDER)
        advanceToPermissions(rider)
        rider.sessionInvalidated()
        assertEquals(OnboardingState.NEEDS_REAUTH_HELP, rider.state)

        val driver = makeOnboardingVM(role = Role.DRIVER)
        advanceToPermissions(driver)
        driver.sessionInvalidated()
        assertEquals(OnboardingState.PHONE_ENTRY, driver.state)
    }

    @Test
    fun launchRoutingResumesLiveSession() {
        assertEquals(OnboardingState.COMPLETE, makeOnboardingVM(hasValidSession = true).state)
        assertEquals(OnboardingState.FRESH_INSTALL, makeOnboardingVM(hasValidSession = false).state)
    }

    @Test
    fun offlineOverlayGatingMatchesCore() {
        // The router only offers the Offline overlay where the core allows it (sign-in / binding).
        assertTrue(allowsOfflineOverlay(OnboardingState.PHONE_ENTRY))
        assertTrue(allowsOfflineOverlay(OnboardingState.DEVICE_BINDING))
        assertFalse(allowsOfflineOverlay(OnboardingState.PERMISSIONS))
        assertFalse(allowsOfflineOverlay(OnboardingState.BELOW_MIN_VERSION))
        assertFalse(allowsOfflineOverlay(OnboardingState.NEEDS_REAUTH_HELP))
    }
}
