package app.boundless.rider.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.boundless.rider.a11y.A11yDescriptor
import app.boundless.rider.a11y.A11yTrait
import app.boundless.rider.i18n.RiderStrings

/**
 * The Rider's Settings rows, as semantic identities. There is deliberately **no** `AUTOMATIC_UPDATES`
 * case: O3 / AC6 require that automatic app updates are an OS-settings concern, never surfaced in
 * Boundless. The absence is therefore a *structural* guarantee — a row for it cannot be constructed —
 * not merely an omission a refactor could undo. Each row's visible title comes from the catalog (P8).
 *
 * The Android twin of `RiderShared.RiderSettingsRow`.
 */
enum class RiderSettingsRow {
    NOTIFICATIONS,
    HELP,
    ;

    fun title(strings: RiderStrings): String = when (this) {
        NOTIFICATIONS -> strings.settingsNotifications
        HELP -> strings.settingsHelp
    }
}

/** The Rider Settings model. [surfacesAutomaticUpdatesToggle] is `false` by construction (AC6). */
class RiderSettingsModel(val rows: List<RiderSettingsRow> = RiderSettingsRow.entries) {
    /** AC6: the Rider Settings surface never offers an automatic-updates toggle. */
    val surfacesAutomaticUpdatesToggle: Boolean = false

    fun a11yReadingOrder(strings: RiderStrings): List<A11yDescriptor> =
        listOf(A11yDescriptor(strings.settingsTitle, setOf(A11yTrait.HEADER))) +
            rows.map { A11yDescriptor(it.title(strings), setOf(A11yTrait.BUTTON)) }
}

/**
 * Renders the Rider Settings list. Rows are navigation affordances (≥48 dp). No toggles at all — in
 * particular, no automatic-updates toggle (AC6). The actual destinations (OS notification settings,
 * a "call your group" sheet) are the deferred app shell.
 */
@Composable
fun RiderSettingsView(strings: RiderStrings, model: RiderSettingsModel = RiderSettingsModel()) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            text = strings.settingsTitle,
            style = MaterialTheme.typography.headlineMedium,
            color = MaterialTheme.colorScheme.onBackground,
            modifier = Modifier.semantics { heading() },
        )
        model.rows.forEach { row ->
            // A clickable surface — a navigational button, NOT a Toggle (AC6 holds by construction).
            // `role = Role.Button` so TalkBack announces it as a button, matching the model's BUTTON
            // trait (the iOS twin uses a real Button, which carries the trait intrinsically).
            Surface(
                onClick = {},
                color = MaterialTheme.colorScheme.surfaceVariant,
                contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(min = 48.dp)
                    .semantics { role = Role.Button },
            ) {
                Text(
                    text = row.title(strings),
                    style = MaterialTheme.typography.bodyLarge,
                    modifier = Modifier.padding(16.dp),
                )
            }
        }
    }
}
