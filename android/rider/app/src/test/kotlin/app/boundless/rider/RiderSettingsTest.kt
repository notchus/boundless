package app.boundless.rider

import app.boundless.rider.settings.RiderSettingsModel
import app.boundless.rider.settings.RiderSettingsRow
import org.junit.Assert.assertFalse
import org.junit.Test

/**
 * AC6 — the Rider's Settings UI does not surface an "automatic updates" toggle (asserts O3). The
 * absence is structural: `RiderSettingsRow` has no such case, so a row for it cannot be built.
 * Twin of `RiderSettingsTests.swift`.
 */
class RiderSettingsTest {
    @Test
    fun noAutomaticUpdatesToggle() {
        assertFalse(RiderSettingsModel().surfacesAutomaticUpdatesToggle)
    }

    @Test
    fun noSettingsRowMentionsAutomaticUpdates() {
        val forbidden = listOf("update", "automatic", "auto-update")
        for (row in RiderSettingsRow.entries) {
            val title = row.title(TestStrings).lowercase()
            for (term in forbidden) {
                assertFalse(
                    "Rider settings row '${row.title(TestStrings)}' must not mention app updates (AC6).",
                    title.contains(term),
                )
            }
        }
    }

    @Test
    fun readingOrderHasNoUpdateAffordance() {
        for (descriptor in RiderSettingsModel().a11yReadingOrder(TestStrings)) {
            assertFalse(descriptor.label.lowercase().contains("update"))
        }
    }
}
