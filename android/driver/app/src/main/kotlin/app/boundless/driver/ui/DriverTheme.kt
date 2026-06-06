package app.boundless.driver.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable

/**
 * The Driver app's Material3 theme — the twin of `:rider:app`'s `RiderTheme` (apps own their theme).
 * Uses the default Material color schemes (no brand palette yet), switching to the dark scheme when
 * the system is in dark mode — so every screen is dark-mode-safe by construction (a11y bar). Type is
 * the Material3 default typography, which scales with the system font scale (200% accessibility text).
 * The composition root (MainActivity, deferred to T14-shell) and the Paparazzi snapshots both wrap
 * content in this theme; the screens read `MaterialTheme.colorScheme`/`typography`, never hardcoded
 * colors or point sizes (a11y bar).
 */
@Composable
fun DriverTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = if (isSystemInDarkTheme()) darkColorScheme() else lightColorScheme(),
        content = content,
    )
}
