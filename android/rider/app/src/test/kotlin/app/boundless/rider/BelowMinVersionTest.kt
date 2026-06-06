package app.boundless.rider

import app.boundless.rider.onboarding.BodyElement
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.boundless_ffi_kotlin.OnboardingEvent
import uniffi.boundless_ffi_kotlin.OnboardingState
import uniffi.boundless_ffi_kotlin.onEvent

/**
 * AC8 (snapshot/UI leg) — a below-`client_min_version` client sees only the calm degradation screen,
 * with no "Update Now" control (asserts O4/O8). The "one alert per member per day" integration leg is
 * the server task T07; the snapshot is in `OnboardingSnapshotTest`. Twin of `BelowMinVersionTests.swift`.
 */
class BelowMinVersionTest {
    @Test
    fun namedScreenRepeatsTheAdminName() {
        val model = testScreens().calmHelp("Sarah")
        val first = model.elements.firstOrNull()
        assertTrue("calm screen should lead with the message", first is BodyElement.Heading)
        val text = (first as BodyElement.Heading).text
        assertEquals("This device needs Sarah's help. Sarah has been told.", text)
        // Repeat the name, never a pronoun (translates correctly across gendered languages).
        assertEquals(2, text.split("Sarah").size - 1)
    }

    @Test
    fun nameLessFallbackWhenNoManifest() {
        val model = testScreens().calmHelp(null)
        assertTrue(model.elements.contains(BodyElement.Heading(TestStrings.belowMinVersionGeneric)))
    }

    @Test
    fun noUpdateNowControl() {
        for (adminName in listOf("Sarah", null)) {
            val model = testScreens().calmHelp(adminName)
            assertTrue("below-min screen must have no CTA (no 'Update Now', O8).", model.actions.isEmpty())
            for (label in model.actionLabels) {
                assertFalse(label.lowercase().contains("update"))
            }
        }
    }

    @Test
    fun reachableFromAnyState() {
        // O4: the degradation is reachable from any auth response / handshake, not only sign-in.
        assertEquals(
            OnboardingState.BELOW_MIN_VERSION,
            onEvent(OnboardingState.PHONE_ENTRY, OnboardingEvent.BelowMinVersionDetected),
        )
        assertEquals(
            OnboardingState.BELOW_MIN_VERSION,
            onEvent(OnboardingState.PERMISSIONS, OnboardingEvent.BelowMinVersionDetected),
        )
    }
}
