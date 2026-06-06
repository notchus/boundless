package app.boundless.rider

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The four name-bearing screens must NOT render a broken sentence when no manifest/admin name is
 * cached (first-launch manifest race / verify failure). With a `null` admin name they resolve a
 * name-less fallback (mirroring `auth_below_min_version_generic`), never an empty `%1$s` slot — so no
 * dangling punctuation reaches an already-confused rider (P10 / voice-and-tone). Twin of
 * `NameLessFallbackTests.swift`.
 */
class NameLessFallbackTest {
    @Test
    fun nameLessAccessorsUseTheGenericFallback() {
        assertEquals(
            "That number doesn't match what's on file. Try again, or your group can help.",
            TestStrings.phoneNotOnFile(null),
        )
        assertEquals("Enter your Onboarding Code.", TestStrings.codePrompt(null))
        assertEquals("That code didn't work. Your group can give you a new one.", TestStrings.codeInvalid(null))
        assertEquals("We'll let your group know notifications aren't on yet.", TestStrings.notificationsDeclined(null))
    }

    @Test
    fun nameLessAccessorsStillSubstituteWhenNamed() {
        assertEquals("Enter the Onboarding Code from Sarah.", TestStrings.codePrompt("Sarah"))
        assertTrue(TestStrings.codeInvalid("Sarah").contains("Sarah"))
    }

    /** No name-bearing screen, with a null admin name, may render a broken sentence: no leftover `%`
     *  format specifier, no empty-slot artifacts (" ." / " ," / "  "). */
    @Test
    fun noScreenRendersDanglingPunctuationWhenNameMissing() {
        val nameless = listOf(
            testScreens().phoneNotOnFile(null) {},
            testScreens().deviceBinding(null) {},
            testScreens().bindingFailed(null) {},
            testScreens().permissionsDeclined(null) {},
        )
        for (model in nameless) {
            for (descriptor in model.a11yReadingOrder) {
                val text = descriptor.label
                assertFalse("Unsubstituted format specifier in: '$text'", text.contains("%"))
                assertFalse("Dangling space-period in: '$text'", text.contains(" ."))
                assertFalse("Dangling space-comma in: '$text'", text.contains(" ,"))
                assertFalse("Double space (empty slot) in: '$text'", text.contains("  "))
            }
        }
    }
}
