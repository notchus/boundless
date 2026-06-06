package app.boundless.rider

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.boundless.rider.onboarding.OnboardingScreenView
import app.boundless.rider.onboarding.PrimarySurfacePlaceholder
import app.boundless.rider.settings.RiderSettingsView
import app.boundless.rider.ui.RiderTheme
import app.cash.paparazzi.DeviceConfig
import app.cash.paparazzi.Paparazzi
import com.android.resources.LayoutDirection
import com.android.resources.NightMode
import org.junit.Rule
import org.junit.Test

/**
 * The a11y snapshot matrix (AC11): every Rider onboarding screen × {default, largest font, dark, RTL}.
 * Also closes the snapshot legs of AC5 (auto-update enabled), AC8 (calm below-min screen, no CTA),
 * AC14 (declined permission), AC15 (NeedsReauthHelp — no form). Strings come from the catalog (P8,
 * resolved via [TestStrings] = the real strings.xml); the screens render the core state machine (P4).
 * The Android twin of `OnboardingSnapshotTests.swift` — 17 screen cases × 4 variants = 68 baselines.
 *
 * `supportsRtl = true` so the RTL variant actually mirrors (the manifest-level flag's Paparazzi twin).
 * Config is varied per-variant with `unsafeUpdateConfig` (1.3.5 has no per-`snapshot` deviceConfig).
 */
class OnboardingSnapshotTest {
    @get:Rule
    val paparazzi = Paparazzi(deviceConfig = DeviceConfig.PIXEL_5, supportsRtl = true)

    private val screens = testScreens()
    private val admin = Fixtures.ADMIN_NAME

    @Test fun helperIntro() = snapshotVariants { OnboardingScreenView(screens.helperIntro {}) }

    @Test fun phoneEntry() = snapshotVariants { OnboardingScreenView(screens.phoneEntry {}) }

    @Test fun phoneEntryOffline() =
        snapshotVariants { OnboardingScreenView(screens.phoneEntry(isOffline = true) {}) }

    @Test fun phoneNotOnFile() = snapshotVariants { OnboardingScreenView(screens.phoneNotOnFile(admin) {}) }

    @Test fun deviceBinding() = snapshotVariants { OnboardingScreenView(screens.deviceBinding(admin) {}) }

    @Test fun deviceBindingOffline() =
        snapshotVariants { OnboardingScreenView(screens.deviceBinding(admin, isOffline = true) {}) }

    /** Reduced screen height + entered text — the reflow proxy for the on-screen keyboard inset (the
     *  OS keyboard itself cannot be captured in a layoutlib snapshot — the AC11 keyboard variant). */
    @Test fun deviceBindingKeyboardInset() =
        snapshotVariants(base = DeviceConfig.PIXEL_5.copy(screenHeight = 1200)) {
            OnboardingScreenView(screens.deviceBinding(admin) {}, fieldValue = "1234")
        }

    @Test fun bindingFailed() = snapshotVariants { OnboardingScreenView(screens.bindingFailed(admin) {}) }

    @Test fun permissions() = snapshotVariants { OnboardingScreenView(screens.permissions({}, {})) }

    @Test fun permissionsDeclined() =
        snapshotVariants { OnboardingScreenView(screens.permissionsDeclined(admin) {}) }

    @Test fun autoUpdateStep() = snapshotVariants { OnboardingScreenView(screens.autoUpdateStep {}) }

    @Test fun autoUpdateEnabled() = snapshotVariants { OnboardingScreenView(screens.autoUpdateEnabled {}) }

    @Test fun belowMinVersionNamed() = snapshotVariants { OnboardingScreenView(screens.calmHelp(admin)) }

    @Test fun belowMinVersionGeneric() = snapshotVariants { OnboardingScreenView(screens.calmHelp(null)) }

    /** Same calm-screen pattern as below-min (AC15) — rendered from the same factory; the router maps
     *  both BELOW_MIN_VERSION and NEEDS_REAUTH_HELP here. */
    @Test fun needsReauthHelp() = snapshotVariants { OnboardingScreenView(screens.calmHelp(admin)) }

    /** Silent completion: the hand-off placeholder, with no "all set" celebration (voice-and-tone). */
    @Test fun primarySurfacePlaceholder() = snapshotVariants { PrimarySurfacePlaceholder() }

    @Test fun riderSettings() = snapshotVariants { RiderSettingsView(TestStrings) }

    // ── Helpers ──────────────────────────────────────────────────────────────────────────────

    private fun snapshotVariants(
        base: DeviceConfig = DeviceConfig.PIXEL_5,
        content: @Composable () -> Unit,
    ) {
        val variants = listOf(
            "default" to base,
            "largestText" to base.copy(fontScale = 2.0f), // 200% — Android's max accessibility text
            "dark" to base.copy(nightMode = NightMode.NIGHT),
            "rtl" to base.copy(layoutDirection = LayoutDirection.RTL),
        )
        for ((suffix, config) in variants) {
            paparazzi.unsafeUpdateConfig(deviceConfig = config)
            paparazzi.snapshot(name = suffix) { Framed(content) }
        }
    }

    @Composable
    private fun Framed(content: @Composable () -> Unit) {
        // Wrap in the app theme + a full-bleed background Surface so dark mode flips the color scheme
        // and the screen renders against the real background (a11y bar: contrast + dark-mode-safe).
        RiderTheme {
            Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                content()
            }
        }
    }
}
