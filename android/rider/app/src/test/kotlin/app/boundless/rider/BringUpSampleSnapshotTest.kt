package app.boundless.rider

import app.cash.paparazzi.DeviceConfig
import app.cash.paparazzi.Paparazzi
import org.junit.Rule
import org.junit.Test

/**
 * Proves the Paparazzi snapshot harness records + verifies green in this module. T13 replaces this
 * with the real onboarding screens and expands to the four a11y variants the bar requires — using
 * the SAME mechanism: a `DeviceConfig` per variant (e.g. `fontScale` for largest Dynamic Type,
 * `nightMode = NightMode.NIGHT` for dark, `layoutDirection = RTL` for RTL). One default-config
 * snapshot here is enough to prove the pipeline; the variants are just more `DeviceConfig` params.
 */
class BringUpSampleSnapshotTest {
    @get:Rule
    val paparazzi = Paparazzi(deviceConfig = DeviceConfig.NEXUS_5)

    @Test
    fun bringUpSample() {
        paparazzi.snapshot { BringUpSample() }
    }
}
