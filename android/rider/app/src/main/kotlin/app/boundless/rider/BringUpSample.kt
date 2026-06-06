package app.boundless.rider

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/**
 * Bring-up harness sample — NOT a shipped screen. It exists only so the Android Paparazzi
 * snapshot harness has something deterministic to record/verify, proving Compose + Material3 +
 * layoutlib + Paparazzi are wired green before T13 adds the real onboarding screens (which render
 * from :core-bridge and carry the ×4 a11y snapshot matrix). Deliberately text-free: no copy means
 * no i18n surface (P8) and no font-metric variance across CI runtimes.
 */
@Composable
fun BringUpSample() {
    MaterialTheme {
        Surface {
            Box(
                Modifier
                    .size(96.dp)
                    .background(MaterialTheme.colorScheme.primary),
            )
        }
    }
}
