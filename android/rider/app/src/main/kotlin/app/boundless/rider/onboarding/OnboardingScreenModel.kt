package app.boundless.rider.onboarding

import app.boundless.rider.a11y.A11yDescriptor
import app.boundless.rider.a11y.A11yTrait

/**
 * A body element of an onboarding screen. Pure data, so the SAME value both renders (in
 * `OnboardingScreenView`) and produces the screen's a11y reading order — the two cannot drift.
 *
 * The Android twin of `RiderShared`'s `BodyElement` (P4). `Code` is defined for parity with the
 * shared kit (the Driver's one-time Recovery Code, spec 001 T14 / AC19); the Rider renders none.
 */
sealed interface BodyElement {
    /** The screen title — rendered large, marked as a TalkBack heading. */
    data class Heading(val text: String) : BodyElement

    /** Explanatory copy. */
    data class Paragraph(val text: String) : BodyElement

    /** A calm info / recovery banner (a bordered container + text — never color-only; a11y bar). */
    data class Banner(val text: String) : BodyElement

    /** A completed-state announcement (e.g. "Automatic updates are on.") — static text, announced
     *  as a state, NOT a button (a11y notes; AC5). */
    data class Confirmation(val text: String) : BodyElement

    /** A prominent value to read and keep (the Driver's Recovery Code) — large, monospaced,
     *  selectable. The string is *data* (from the server), passed verbatim, never localized. */
    data class Code(val text: String) : BodyElement
}

/** A tappable affordance — the single large control(s) per step (a11y notes). Holds a closure. */
class ScreenAction(
    val label: String,
    val emphasis: Emphasis = Emphasis.PRIMARY,
    val onClick: () -> Unit,
) {
    enum class Emphasis { PRIMARY, SECONDARY }
}

/**
 * An optional single text entry (phone number / Onboarding Code). The live value + onValueChange are
 * hoisted to the renderer/router (Compose state hoisting), so the model itself stays pure and
 * comparable. `isOffline` (on the model) renders the action disabled — the Offline overlay: the
 * sign-in UI is shown but the network-dependent action is deferred until connectivity, so the
 * overlay reuses the sign-in copy and adds no new catalog keys.
 */
data class FieldModel(val label: String, val kind: Kind) {
    enum class Kind { PHONE, CODE }
}

/** The complete description of one onboarding screen: what to render AND (derived) its a11y order. */
class OnboardingScreenModel(
    val elements: List<BodyElement>,
    val field: FieldModel? = null,
    val actions: List<ScreenAction> = emptyList(),
    /** Offline overlay over a sign-in/binding step: the action is shown disabled (deferred until
     *  connectivity). Only set where `allowsOfflineOverlay(state)` (the core) is true. */
    val isOffline: Boolean = false,
) {
    /** The TalkBack reading order, derived from the rendered content (no drift). Tests assert it:
     *  body elements in order → the field (if any) → the actions in order. */
    val a11yReadingOrder: List<A11yDescriptor>
        get() {
            val order = elements.mapTo(mutableListOf()) { element ->
                when (element) {
                    is BodyElement.Heading -> A11yDescriptor(element.text, setOf(A11yTrait.HEADER))
                    is BodyElement.Paragraph -> A11yDescriptor(element.text, setOf(A11yTrait.STATIC_TEXT))
                    is BodyElement.Banner -> A11yDescriptor(element.text, setOf(A11yTrait.STATIC_TEXT))
                    is BodyElement.Confirmation -> A11yDescriptor(element.text, setOf(A11yTrait.STATIC_TEXT))
                    is BodyElement.Code -> A11yDescriptor(element.text, setOf(A11yTrait.STATIC_TEXT))
                }
            }
            // `this.field` — inside a getter the bare `field` is the backing-field soft keyword.
            this.field?.let { order += A11yDescriptor(it.label, setOf(A11yTrait.TEXT_FIELD)) }
            order += actions.map { A11yDescriptor(it.label, setOf(A11yTrait.BUTTON)) }
            return order
        }

    /** Whether this screen exposes any text-entry / sign-in affordance. AC15 (NeedsReauthHelp shows
     *  no form) and AC1(b) (no signup route) assert this is `false` for the calm/terminal screens. */
    val hasInputAffordance: Boolean get() = this.field != null

    /** All affordance labels on the screen — used by AC8 ("no Update Now") and AC1(b) inspections. */
    val actionLabels: List<String> get() = actions.map { it.label }
}
