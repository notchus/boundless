package app.boundless.rider

import app.boundless.rider.onboarding.BodyElement
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * AC5 — the first-launch flow contains an auto-update step and a screen labeled "auto-update enabled"
 * (asserts O3). The snapshot leg is in `OnboardingSnapshotTest`; this asserts the copy resolves and
 * the confirmation screen carries the labeled, completed-state element. Twin of `AutoUpdateStepTests.swift`.
 */
class AutoUpdateTest {
    @Test
    fun autoUpdateEnabledScreenIsLabeled() {
        assertEquals("Automatic updates are on.", TestStrings.autoUpdateEnabled)

        val model = testScreens().autoUpdateEnabled {}
        // The "auto-update enabled" label is present as a completed-state confirmation element.
        assertTrue(model.elements.contains(BodyElement.Confirmation(TestStrings.autoUpdateEnabled)))
    }

    @Test
    fun autoUpdateStepPresentsTheStep() {
        assertEquals("Turn on automatic updates.", TestStrings.autoUpdateStep)
        val model = testScreens().autoUpdateStep {}
        assertTrue(model.elements.contains(BodyElement.Heading(TestStrings.autoUpdateStep)))
    }
}
