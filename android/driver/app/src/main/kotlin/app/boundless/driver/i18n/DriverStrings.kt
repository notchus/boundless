package app.boundless.driver.i18n

import app.boundless.rider.i18n.RiderStrings

/**
 * Every user-visible Driver onboarding string, resolved from the catalogs. [DriverStrings] **extends**
 * the shared [RiderStrings] (the Android twin of iOS's `L10n` + `DriverL10n`): a Driver screen factory
 * thus reads both the shared accessors it reuses (`actionContinue`, `phonePrompt`, `signInAgain`) and
 * the four Driver-only ones below from a single resolver. Views/a11y labels call these typed accessors
 * — never a raw key or English literal (constitution P8).
 *
 * The shared keys come from `:rider:shared`'s merged catalog; the four Driver keys from this module's
 * `res/values/strings.xml`. The test/snapshot resolver `CatalogDriverStrings` parses both files (the
 * single source of truth); the production `AndroidDriverStrings` (over Android `Resources`) is the
 * deferred app shell.
 */
interface DriverStrings : RiderStrings {
    /** Driver self-onboard intro (FreshInstall) — self-directed, vs the Rider's helper-facing copy. */
    val driverIntro: String get() = string(DriverKeys.DRIVER_INTRO)

    // ── One-time Recovery Code capture (ADR-0016 D3 / AC19) ───────────────────────────────
    val recoveryTitle: String get() = string(DriverKeys.RECOVERY_TITLE)
    val recoveryExplanation: String get() = string(DriverKeys.RECOVERY_EXPLANATION)
    val recoverySaved: String get() = string(DriverKeys.RECOVERY_SAVED)
}

/**
 * Driver catalog keys = the Driver `res/values/strings.xml` resource names (dot→underscore from the
 * iOS `DriverOnboarding.xcstrings` keys). Single-sourced here so the impls and the strings.xml stay in
 * lock-step. The shared keys live in `:rider:shared`'s `Keys`.
 */
object DriverKeys {
    const val DRIVER_INTRO = "onboarding_driver_intro"
    const val RECOVERY_TITLE = "onboarding_recovery_title"
    const val RECOVERY_EXPLANATION = "onboarding_recovery_explanation"
    const val RECOVERY_SAVED = "onboarding_recovery_saved"
}
