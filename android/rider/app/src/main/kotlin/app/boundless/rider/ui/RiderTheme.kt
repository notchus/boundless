package app.boundless.rider.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable

/**
 * The Rider app's Material3 theme. Uses the default Material color schemes (no brand palette yet),
 * switching to the dark scheme when the system is in dark mode — so every screen is dark-mode-safe
 * by construction (a11y bar). Type is the Material3 default typography, which scales with the system
 * font scale (Dynamic Type / 200% accessibility text). The composition root (MainActivity, deferred
 * to T13-shell) and the Paparazzi snapshots both wrap content in this theme; the screens themselves
 * read `MaterialTheme.colorScheme`/`typography`, never hardcoded colors or point sizes (a11y bar).
 */
@Composable
fun RiderTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = if (isSystemInDarkTheme()) darkColorScheme() else lightColorScheme(),
        content = content,
    )
}
