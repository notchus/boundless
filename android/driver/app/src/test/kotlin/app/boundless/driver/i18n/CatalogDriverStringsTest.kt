package app.boundless.driver.i18n

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Proves the Driver's merged catalog resolver parses BOTH the shared `:rider:shared` strings.xml and
 * the Driver's own strings.xml + unescapes + substitutes positional args — so every other Driver test
 * (and the snapshots) resolves the genuine shipped copy across both catalogs. Guards the
 * single-source-of-truth invariant (no English drift).
 */
class CatalogDriverStringsTest {
    private val strings = CatalogDriverStrings.fromDefaultCatalog()

    /** The 4 Driver-only keys resolve from the Driver strings.xml, apostrophes unescaped. */
    @Test
    fun resolvesDriverKeys() {
        assertEquals("Let's get you set up.", strings.driverIntro)
        assertEquals("Save your Recovery Code.", strings.recoveryTitle)
        assertEquals(
            "You'll need this to set up Boundless on a new phone. Keep it somewhere safe.",
            strings.recoveryExplanation,
        )
        assertEquals("I've saved it", strings.recoverySaved)
    }

    /** The shared keys (inherited from RiderStrings) resolve from the :rider:shared catalog — so a
     *  Driver screen factory reads both catalogs through one resolver. */
    @Test
    fun resolvesSharedKeysIncludingSignInAgain() {
        assertEquals("Continue", strings.actionContinue)
        assertEquals("What's the phone number on file?", strings.phonePrompt)
        assertEquals("Let's sign in again. Your phone number works.", strings.signInAgain)
    }

    /** Shared positional substitution + the generic fallback still work through DriverStrings. */
    @Test
    fun substitutesAndFallsBackThroughSharedAccessors() {
        assertEquals("Enter the Onboarding Code from Sarah.", strings.codePrompt("Sarah"))
        assertTrue(strings.belowMinVersion("Sarah").contains("Sarah"))
        assertEquals("Enter your Onboarding Code.", strings.codePrompt(null))
    }
}
