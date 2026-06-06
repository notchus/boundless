package app.boundless.rider.a11y

/**
 * One element in a screen's TalkBack reading order: its label and accessibility traits.
 *
 * A screen derives its `a11yReadingOrder: List<A11yDescriptor>` from the SAME
 * [app.boundless.rider.onboarding.OnboardingScreenModel] it renders, so the asserted reading order
 * cannot drift from what is drawn. Tests assert this list (labels present, headings marked, the
 * auto-update confirmation announced as a completed *state* not a button — AC11). Paparazzi has no
 * semantics-tree assertion strategy, so this model-level check IS the automatable reading-order
 * leg; the full recorded TalkBack walkthrough remains a manual checklist item (plan §7 / DEFERRED).
 *
 * The Android twin of `RiderShared/A11y/A11yDescriptor.swift` (P4 — same model on every platform).
 */
enum class A11yTrait { HEADER, STATIC_TEXT, BUTTON, TEXT_FIELD }

data class A11yDescriptor(val label: String, val traits: Set<A11yTrait>) {
    val isHeader: Boolean get() = A11yTrait.HEADER in traits
    val isButton: Boolean get() = A11yTrait.BUTTON in traits
    val isStaticText: Boolean get() = A11yTrait.STATIC_TEXT in traits
}
