package app.boundless.driver

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.boundless_ffi_kotlin.OnboardingState
import uniffi.boundless_ffi_kotlin.Role
import uniffi.boundless_ffi_kotlin.reauthStateFor

/**
 * AC15 (Driver branch) — a Driver whose session expired is routed to **interactive re-auth** at
 * `PhoneEntry` (showing `auth.signin_again`), unlike a Rider, who sees the form-less calm
 * `NeedsReauthHelp`. The divergence is the core's decision (`reauthStateFor`), not the UI's (P4).
 * The Android twin of `DriverReauthRouteTests.swift`.
 */
class DriverReauthRouteTest {
    /** The core routes Driver vs Rider differently — this is the whole point of the branch (FFI). */
    @Test
    fun coreRoutesDriverToPhoneEntryAndRiderToHelp() {
        assertEquals(OnboardingState.PHONE_ENTRY, reauthStateFor(Role.DRIVER))
        assertEquals(OnboardingState.NEEDS_REAUTH_HELP, reauthStateFor(Role.RIDER))
    }

    /** An invalidated Driver session lands the VM at `PhoneEntry` and records `reauthRequested`, so
     *  the router shows the re-auth variant rather than the fresh sign-in. The target state is the
     *  core's (`reauthStateFor(DRIVER)`); the flag only records that it happened. */
    @Test
    fun sessionInvalidatedRoutesToPhoneEntryReauth() {
        val vm = makeDriverVM()
        vm.begin() // FreshInstall → PhoneEntry, then drive into the flow before invalidation
        assertFalse(vm.reauthRequested)

        vm.sessionInvalidated()
        assertEquals(OnboardingState.PHONE_ENTRY, vm.state)
        assertTrue(vm.reauthRequested)
    }

    /** The re-auth screen leads with `auth.signin_again` and **is** a sign-in form (a Driver
     *  self-re-auths) — the deliberate contrast with the Rider's form-less calm screen. */
    @Test
    fun reAuthScreenIsASignInFormLedBySignInAgain() {
        val model = driverScreens().reAuthPhoneEntry {}

        assertEquals(TestStrings.signInAgain, model.a11yReadingOrder.first().label)
        assertEquals("Re-auth leads with a header.", true, model.a11yReadingOrder.first().isHeader)
        assertTrue("A Driver re-auths interactively — this IS a form.", model.hasInputAffordance)

        // Contrast: a Rider in the same situation gets the form-less calm screen.
        assertFalse(riderScreens().calmHelp(Fixtures.ADMIN_NAME).hasInputAffordance)
    }
}
