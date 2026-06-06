package app.boundless.driver

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * AC11 (reading-order leg) — every Driver screen has a complete, ordered TalkBack reading: non-empty
 * labels, a leading header where the screen has a title, the Recovery Code announced as static text
 * (a value to read, not a control), and the auto-update confirmation announced as a completed *state*,
 * not a button. Paparazzi has no semantics-tree strategy, so this model-level assertion is the
 * automatable order check; the recorded TalkBack walkthrough remains a manual checklist item
 * (DEFERRED). Twin of `VoiceOverTraversalTests.swift`.
 */
class TalkBackTraversalTest {
    @Test
    fun everyScreenHasLabeledReadingOrder() {
        for ((name, model) in DriverScreenFixtures.allModels()) {
            val order = model.a11yReadingOrder
            assertTrue("Screen '$name' has an empty reading order.", order.isNotEmpty())
            for (descriptor in order) {
                assertTrue(
                    "Screen '$name' has an unlabeled element (TalkBack would read nothing).",
                    descriptor.label.trim().isNotEmpty(),
                )
                assertTrue("Screen '$name' element has no trait.", descriptor.traits.isNotEmpty())
            }
        }
    }

    @Test
    fun headerLeadsEachTitledScreen() {
        val titled = listOf(
            "driverIntro", "reAuthPhoneEntry", "recoveryCodeCapture",
            "phoneEntry", "deviceBinding", "permissions", "belowMinVersionNamed",
        )
        val models = DriverScreenFixtures.allModels().toMap()
        for (name in titled) {
            assertEquals(
                "Screen '$name' should lead with a header.",
                true,
                models[name]?.a11yReadingOrder?.first()?.isHeader,
            )
        }
    }

    /** The Recovery Code capture reads, in order: title (header) → explanation → code (static) →
     *  the confirm button. The code is read as a value, never announced as a control. */
    @Test
    fun recoveryCaptureReadingOrder() {
        val order = driverScreens().recoveryCodeCapture(Fixtures.RECOVERY_CODE) {}.a11yReadingOrder
        assertEquals(4, order.size)
        assertEquals(TestStrings.recoveryTitle, order[0].label)
        assertTrue(order[0].isHeader)
        assertEquals(TestStrings.recoveryExplanation, order[1].label)
        assertTrue(order[1].isStaticText)
        assertEquals(Fixtures.RECOVERY_CODE, order[2].label)
        assertTrue("The code is read as static text, not a control.", order[2].isStaticText)
        assertEquals(TestStrings.recoverySaved, order[3].label)
        assertTrue(order[3].isButton)
    }

    @Test
    fun autoUpdateEnabledIsAnnouncedAsStateNotButton() {
        val model = riderScreens().autoUpdateEnabled {}
        val confirmation = model.a11yReadingOrder.firstOrNull { it.label == TestStrings.autoUpdateEnabled }
        assertNotNull(confirmation)
        assertEquals(true, confirmation?.isStaticText)
        assertEquals("'auto-update enabled' must not be a button (a11y).", false, confirmation?.isButton)
    }
}
