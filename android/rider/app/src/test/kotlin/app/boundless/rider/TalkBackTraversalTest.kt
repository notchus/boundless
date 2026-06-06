package app.boundless.rider

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * AC11 (reading-order leg) — every screen has a complete, ordered TalkBack reading: non-empty labels,
 * a leading header where the screen has a title, and the auto-update confirmation announced as a
 * completed *state*, not a button. Paparazzi has no semantics-tree strategy, so this model-level
 * assertion is the automatable order check; the recorded TalkBack walkthrough remains a manual
 * checklist item (plan §7 / DEFERRED). Twin of `VoiceOverTraversalTests.swift`.
 */
class TalkBackTraversalTest {
    @Test
    fun everyScreenHasLabeledReadingOrder() {
        for ((name, model) in ScreenFixtures.allModels()) {
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
        val titled = listOf("helperIntro", "phoneEntry", "deviceBinding", "permissions", "belowMinVersionNamed")
        val models = ScreenFixtures.allModels().toMap()
        for (name in titled) {
            assertEquals(
                "Screen '$name' should lead with a header.",
                true,
                models[name]?.a11yReadingOrder?.first()?.isHeader,
            )
        }
    }

    @Test
    fun autoUpdateEnabledIsAnnouncedAsStateNotButton() {
        val model = testScreens().autoUpdateEnabled {}
        val confirmation = model.a11yReadingOrder.firstOrNull { it.label == TestStrings.autoUpdateEnabled }
        assertNotNull(confirmation)
        assertEquals(true, confirmation?.isStaticText)
        assertEquals("'auto-update enabled' must not be a button (a11y).", false, confirmation?.isButton)
    }
}
