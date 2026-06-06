package app.boundless.rider

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.boundless_ffi_kotlin.OnboardingState
import uniffi.boundless_ffi_kotlin.Role
import uniffi.boundless_ffi_kotlin.reauthStateFor

/**
 * AC15 / P10 — a Rider whose session expired sees the calm `NeedsReauthHelp` screen: no phone-entry
 * field, no sign-in form. (The "one alert per member per day" integration leg is the server task T07;
 * the snapshot leg is in `OnboardingSnapshotTest`.) Twin of `NeedsReauthHelpTests.swift`.
 */
class NeedsReauthHelpTest {
    @Test
    fun calmScreenHasNoFormOrCTA() {
        for (adminName in listOf(Fixtures.ADMIN_NAME, null)) {
            val model = testScreens().calmHelp(adminName)
            assertFalse("NeedsReauthHelp must show no field (AC15).", model.hasInputAffordance)
            assertTrue("NeedsReauthHelp must show no sign-in / CTA (AC15).", model.actions.isEmpty())
        }
    }

    @Test
    fun loneRiderRoutesToCalmScreenNotAForm() {
        // The core routes a lone Rider to the calm help screen (and a Driver to interactive re-auth).
        assertEquals(OnboardingState.NEEDS_REAUTH_HELP, reauthStateFor(Role.RIDER))
        assertEquals(OnboardingState.PHONE_ENTRY, reauthStateFor(Role.DRIVER))
    }
}
