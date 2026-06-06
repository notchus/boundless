package app.boundless.rider.onboarding

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp

/**
 * Renders an [OnboardingScreenModel]. One renderer for every onboarding screen, so layout, font
 * scaling, contrast and dark mode are consistent and correct by construction (a11y bar). Uses
 * Material3 **semantic colors** (`MaterialTheme.colorScheme`, dark-mode-safe) and **type styles**
 * (`MaterialTheme.typography`, which scale with the system font scale) — never hardcoded colors or
 * point sizes. The `verticalScroll` lets every screen reflow at the largest font scale without
 * clipping. Interactive controls are Material3 components with an explicit ≥48 dp touch target.
 *
 * The Android twin of `RiderShared.OnboardingScreenView`. The field's live value + change callback
 * are hoisted (Compose state hoisting) so the model stays pure; the router owns that state.
 */
@Composable
fun OnboardingScreenView(
    model: OnboardingScreenModel,
    modifier: Modifier = Modifier,
    fieldValue: String = "",
    onFieldValueChange: (String) -> Unit = {},
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        model.elements.forEach { ElementView(it) }
        model.field?.let { FieldView(it, fieldValue, onFieldValueChange, enabled = !model.isOffline) }
        model.actions.forEach { ActionButton(it, enabled = !model.isOffline) }
    }
}

@Composable
private fun ElementView(element: BodyElement) {
    when (element) {
        is BodyElement.Heading ->
            Text(
                text = element.text,
                style = MaterialTheme.typography.headlineMedium,
                color = MaterialTheme.colorScheme.onBackground,
                modifier = Modifier.semantics { heading() },
            )

        is BodyElement.Paragraph ->
            Text(
                text = element.text,
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onBackground,
            )

        is BodyElement.Banner ->
            // A bordered, tinted container + text — distinguished by SHAPE, never color alone
            // (a11y bar); the message lives in the words, calm not alarming. Deliberate platform
            // divergence from iOS (which adds an info.circle icon): Material3 shape+tint+text already
            // satisfies "not color-only", so we avoid pulling material-icons-extended (a large dep)
            // just for a decorative glyph. Optional icon parity is a T13-shell note (DEFERRED).
            Surface(
                color = MaterialTheme.colorScheme.secondaryContainer,
                contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
                shape = RoundedCornerShape(12.dp),
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(
                    text = element.text,
                    style = MaterialTheme.typography.bodyLarge,
                    modifier = Modifier.padding(16.dp),
                )
            }

        is BodyElement.Confirmation ->
            // A completed state — static text, NOT a button (a11y notes; AC5). The words carry the
            // meaning; rendered prominently but it is not a control. Deliberate divergence from iOS
            // (which adds a checkmark.circle): the text alone conveys the completed state and is
            // announced as static text, so no icon dep is pulled (optional parity → DEFERRED).
            Text(
                text = element.text,
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.onBackground,
            )

        is BodyElement.Code ->
            // A prominent value to read and keep (the Recovery Code). Monospaced, high-contrast,
            // bordered; scales with the font scale. Static text — not a control (a11y bar).
            Text(
                text = element.text,
                style = MaterialTheme.typography.headlineSmall.copy(fontFamily = FontFamily.Monospace),
                color = MaterialTheme.colorScheme.onSurface,
                modifier = Modifier
                    .fillMaxWidth()
                    .border(1.dp, MaterialTheme.colorScheme.outline, RoundedCornerShape(12.dp))
                    .padding(16.dp),
            )
    }
}

@Composable
private fun FieldView(
    field: FieldModel,
    value: String,
    onValueChange: (String) -> Unit,
    enabled: Boolean,
) {
    // No visible Material label — the heading above already shows this exact prompt, so a floating
    // label would duplicate it. `contentDescription` gives the editable field its accessible NAME
    // (the analog of iOS's `accessibilityLabel`); the recorded TalkBack walkthrough (DEFERRED, manual)
    // confirms it announces as an editable text box with this name. Input-security hardening for the
    // Onboarding Code field (one-time-code autofill content type + no-personalized-learning, a Compose
    // 1.8 API) lands with the real input flow in T13-shell — see DEFERRED.
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        enabled = enabled,
        singleLine = true,
        keyboardOptions = KeyboardOptions(
            keyboardType = if (field.kind == FieldModel.Kind.PHONE) KeyboardType.Phone else KeyboardType.Number,
        ),
        modifier = Modifier
            .fillMaxWidth()
            .semantics { contentDescription = field.label },
    )
}

@Composable
private fun ActionButton(action: ScreenAction, enabled: Boolean) {
    val mod = Modifier
        .fillMaxWidth()
        .heightIn(min = 48.dp)
    when (action.emphasis) {
        ScreenAction.Emphasis.PRIMARY ->
            Button(onClick = action.onClick, enabled = enabled, modifier = mod) {
                Text(action.label, style = MaterialTheme.typography.titleMedium)
            }

        ScreenAction.Emphasis.SECONDARY ->
            OutlinedButton(onClick = action.onClick, enabled = enabled, modifier = mod) {
                Text(action.label, style = MaterialTheme.typography.titleMedium)
            }
    }
}

/**
 * The hand-off after a **silent** onboarding completion. The real Rider primary surface ("You're
 * coming tonight") is a later spec; this neutral placeholder asserts the constitution's "no
 * celebration of plumbing" — there is deliberately no "all set" copy here, and it is hidden from
 * accessibility (`clearAndSetSemantics`), exactly like `RiderShared.PrimarySurfacePlaceholderView`.
 */
@Composable
fun PrimarySurfacePlaceholder(modifier: Modifier = Modifier) {
    Box(
        modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
            .clearAndSetSemantics {},
    )
}
