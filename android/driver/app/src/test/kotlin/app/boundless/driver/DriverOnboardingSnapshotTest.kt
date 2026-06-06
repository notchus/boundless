package app.boundless.driver

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.boundless.driver.ui.DriverTheme
import app.boundless.rider.onboarding.OnboardingScreenView
import app.boundless.rider.onboarding.PrimarySurfacePlaceholder
import app.cash.paparazzi.DeviceConfig
import app.cash.paparazzi.Paparazzi
import com.android.resources.LayoutDirection
import com.android.resources.NightMode
import org.junit.Rule
import org.junit.Test

/**
 * The a11y snapshot matrix (AC11): **every Driver onboarding screen** × {default, largest font, dark,
 * RTL}. The Driver reuses `:rider:shared`'s renderer for the role-neutral steps, so those baselines
 * render identically to the Rider's — but they are the **Driver app's own** baselines, independently
 * closing AC11 for this platform (if the shared renderer changes, both apps' baselines update
 * together, which is correct). Also closes the snapshot legs of AC5 (auto-update enabled), AC8 (calm
 * below-min, no CTA), AC14 (declined permission) and AC19 (Recovery Code capture). Strings come from
 * the catalogs (P8, via [TestStrings] = the real merged strings.xml); the screens render the core
 * state machine (P4). The Android twin of `DriverOnboardingSnapshotTests.swift` — 19 screen cases ×
 * 4 variants = 76 baselines.
 *
 * `supportsRtl = true` so the RTL variant actually mirrors; config is varied per-variant with
 * `unsafeUpdateConfig` (Paparazzi 1.3.5 has no per-`snapshot` deviceConfig).
 */
class DriverOnboardingSnapshotTest {
    @get:Rule
    val paparazzi = Paparazzi(deviceConfig = DeviceConfig.PIXEL_5, supportsRtl = true)

    private val driver = driverScreens()
    private val rider = riderScreens()
    private val admin = Fixtures.ADMIN_NAME

    // ── Driver-specific deltas ────────────────────────────────────────────────────────────────

    @Test fun driverIntro() = snapshotVariants { OnboardingScreenView(driver.driverIntro {}) }

    @Test fun reAuthPhoneEntry() = snapshotVariants { OnboardingScreenView(driver.reAuthPhoneEntry {}) }

    /** AC19 — the one-time Recovery Code capture (the code rendered via the prominent `Code` element). */
    @Test fun recoveryCodeCapture() =
        snapshotVariants { OnboardingScreenView(driver.recoveryCodeCapture(Fixtures.RECOVERY_CODE) {}) }

    // ── Reused role-neutral steps (Driver app's own baselines) ──────────────────────────────────

    @Test fun phoneEntry() = snapshotVariants { OnboardingScreenView(rider.phoneEntry {}) }

    @Test fun phoneEntryOffline() =
        snapshotVariants { OnboardingScreenView(rider.phoneEntry(isOffline = true) {}) }

    @Test fun phoneNotOnFile() = snapshotVariants { OnboardingScreenView(rider.phoneNotOnFile(admin) {}) }

    @Test fun phoneNotOnFileNil() = snapshotVariants { OnboardingScreenView(rider.phoneNotOnFile(null) {}) }

    @Test fun deviceBinding() = snapshotVariants { OnboardingScreenView(rider.deviceBinding(admin) {}) }

    @Test fun deviceBindingOffline() =
        snapshotVariants { OnboardingScreenView(rider.deviceBinding(admin, isOffline = true) {}) }

    @Test fun bindingFailed() = snapshotVariants { OnboardingScreenView(rider.bindingFailed(admin) {}) }

    @Test fun bindingFailedNil() = snapshotVariants { OnboardingScreenView(rider.bindingFailed(null) {}) }

    @Test fun permissions() = snapshotVariants { OnboardingScreenView(rider.permissions({}, {})) }

    @Test fun permissionsDeclined() =
        snapshotVariants { OnboardingScreenView(rider.permissionsDeclined(admin) {}) }

    @Test fun permissionsDeclinedNil() =
        snapshotVariants { OnboardingScreenView(rider.permissionsDeclined(null) {}) }

    @Test fun autoUpdateStep() = snapshotVariants { OnboardingScreenView(rider.autoUpdateStep {}) }

    @Test fun autoUpdateEnabled() = snapshotVariants { OnboardingScreenView(rider.autoUpdateEnabled {}) }

    @Test fun belowMinVersionNamed() = snapshotVariants { OnboardingScreenView(rider.calmHelp(admin)) }

    @Test fun belowMinVersionGeneric() = snapshotVariants { OnboardingScreenView(rider.calmHelp(null)) }

    /** Silent completion: the shared hand-off placeholder, with no "all set" celebration. */
    @Test fun primarySurfacePlaceholder() = snapshotVariants { PrimarySurfacePlaceholder() }

    // ── Helpers (mirror :rider:app's OnboardingSnapshotTest) ────────────────────────────────────

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
        DriverTheme {
            Surface(modifier = Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                content()
            }
        }
    }
}
