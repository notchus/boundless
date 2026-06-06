package app.boundless.rider.i18n

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Proves the test/snapshot catalog resolver parses the real strings.xml + unescapes + substitutes
 * positional args correctly — so every other test (and the snapshots) resolves the genuine shipped
 * copy. Guards the single-source-of-truth invariant (no English drift between tests and strings.xml).
 */
class CatalogRiderStringsTest {
    private val strings = CatalogRiderStrings.fromDefaultCatalog()

    @Test
    fun resolvesPlainCopyWithApostrophesUnescaped() {
        assertEquals("Let's set up this phone together.", strings.helperIntro)
        assertEquals("What's the phone number on file?", strings.phonePrompt)
        assertEquals("Automatic updates are on.", strings.autoUpdateEnabled)
        assertEquals("Turn on automatic updates.", strings.autoUpdateStep)
    }

    @Test
    fun substitutesAndRepeatsThePositionalName() {
        assertEquals(
            "This device needs Sarah's help. Sarah has been told.",
            strings.belowMinVersion("Sarah"),
        )
        assertEquals("Enter the Onboarding Code from Sarah.", strings.codePrompt("Sarah"))
    }

    @Test
    fun nullAdminNameSelectsTheGenericFallback() {
        assertEquals("This device needs your group's help. They've been told.", strings.belowMinVersionGeneric)
        assertEquals("Enter your Onboarding Code.", strings.codePrompt(null))
        assertEquals(
            "That number doesn't match what's on file. Try again, or your group can help.",
            strings.phoneNotOnFile(null),
        )
    }

    @Test
    fun unescapeHandlesAndroidEscapes() {
        assertEquals("Let's go", CatalogRiderStrings.unescape("""Let\'s go"""))
        assertEquals("a\nb", CatalogRiderStrings.unescape("""a\nb"""))
        assertEquals("  spaced  ", CatalogRiderStrings.unescape(""""  spaced  """"))
        assertEquals("100%", CatalogRiderStrings.unescape("100%"))
    }
}
