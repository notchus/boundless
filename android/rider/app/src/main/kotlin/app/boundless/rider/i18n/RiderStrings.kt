package app.boundless.rider.i18n

/**
 * Every user-visible Rider onboarding string, resolved from the catalog (`res/values/strings.xml`).
 * Views and a11y labels call these typed accessors — they never hold a raw key or an English literal
 * (constitution P8). The `{adminName}` placeholders are positional `%1$s`; the substitution and the
 * **generic-fallback selection** (a `null` admin name → the `*_generic` key, never an empty slot)
 * are centralized here so the two impls cannot diverge.
 *
 * The Android twin of `RiderShared.L10n`. Two impls back this interface, each supplying only
 * [string]: the production `AndroidRiderStrings` (over Android `Resources`, the deferred T13-shell)
 * and the test/snapshot `CatalogRiderStrings` (which parses the same `strings.xml` — single source
 * of truth, mirroring how iOS tests resolve via the real catalog).
 *
 * Two keys live in the catalog for completeness (AC12) but are rendered by a different surface, so
 * they have no accessor here: the two `admin_onboarding_*` keys (SvelteKit admin UI, spec 001 T15).
 * `signInAgain` exists for parity with the shared kit (the Driver app, spec 001 T14, renders it);
 * the Rider never does.
 */
interface RiderStrings {
    /** Resolve [key] (a `res/values/strings.xml` resource name) with positional `%1$s` [args]. */
    fun string(key: String, vararg args: Any): String

    // ── Affordances (the single large control per step — a11y notes) ─────────────────────
    val actionContinue: String get() = string(Keys.ACTION_CONTINUE)
    val actionTryAgain: String get() = string(Keys.ACTION_TRY_AGAIN)

    // ── Helper intro (FreshInstall) ──────────────────────────────────────────────────────
    val helperIntro: String get() = string(Keys.HELPER_INTRO)

    // ── Sign-in (PhoneEntry / PhoneNotOnFile) ────────────────────────────────────────────
    val phonePrompt: String get() = string(Keys.PHONE_PROMPT)

    /** `null` adminName → the name-less fallback (no manifest cached), never an empty `%1$s` slot. */
    fun phoneNotOnFile(adminName: String?): String =
        if (adminName == null) string(Keys.PHONE_NOT_ON_FILE_GENERIC)
        else string(Keys.PHONE_NOT_ON_FILE, adminName)

    // ── Device binding (Onboarding Code / BindingFailed) ─────────────────────────────────
    fun codePrompt(adminName: String?): String =
        if (adminName == null) string(Keys.CODE_PROMPT_GENERIC) else string(Keys.CODE_PROMPT, adminName)

    fun codeInvalid(adminName: String?): String =
        if (adminName == null) string(Keys.CODE_INVALID_GENERIC) else string(Keys.CODE_INVALID, adminName)

    // ── Permissions (+ declined) ─────────────────────────────────────────────────────────
    val notificationsWhy: String get() = string(Keys.NOTIFICATIONS_WHY)
    val notificationsAllow: String get() = string(Keys.NOTIFICATIONS_ALLOW)
    val notificationsDecline: String get() = string(Keys.NOTIFICATIONS_DECLINE)

    fun notificationsDeclined(adminName: String?): String =
        if (adminName == null) string(Keys.NOTIFICATIONS_DECLINED_GENERIC)
        else string(Keys.NOTIFICATIONS_DECLINED, adminName)

    // ── Auto-update (step + enabled confirmation) ────────────────────────────────────────
    val autoUpdateStep: String get() = string(Keys.AUTOUPDATE_STEP)
    val autoUpdateEnabled: String get() = string(Keys.AUTOUPDATE_ENABLED)

    // ── Re-auth / degradation ────────────────────────────────────────────────────────────
    /** Driver interactive re-auth entry (rendered by the Driver app, T14). Riders never see it —
     *  a lone Rider gets the form-less calm screen (AC15). Present for parity with the shared kit. */
    val signInAgain: String get() = string(Keys.SIGNIN_AGAIN)

    /** O4 / `NeedsReauthHelp` calm screen. The name is repeated (never a pronoun) — both `%1$s`. */
    fun belowMinVersion(adminName: String): String = string(Keys.BELOW_MIN_VERSION, adminName)

    /** Name-less fallback when no manifest/admin name is available (offline first launch). */
    val belowMinVersionGeneric: String get() = string(Keys.BELOW_MIN_VERSION_GENERIC)

    // ── Rider settings (AC6: NO automatic-updates toggle) ────────────────────────────────
    val settingsTitle: String get() = string(Keys.SETTINGS_TITLE)
    val settingsNotifications: String get() = string(Keys.SETTINGS_NOTIFICATIONS)
    val settingsHelp: String get() = string(Keys.SETTINGS_HELP)
}

/**
 * Catalog keys = the `res/values/strings.xml` resource names (dot→underscore from the iOS catalog
 * keys). Single-sourced here so both impls and the strings.xml stay in lock-step. The two
 * `admin_onboarding_*` keys exist only in `strings.xml` (AC12 completeness) — no accessor.
 */
object Keys {
    const val ACTION_CONTINUE = "onboarding_action_continue"
    const val ACTION_TRY_AGAIN = "onboarding_action_try_again"
    const val HELPER_INTRO = "onboarding_helper_intro"
    const val PHONE_PROMPT = "onboarding_signin_phone_prompt"
    const val PHONE_NOT_ON_FILE = "onboarding_signin_phone_not_on_file"
    const val PHONE_NOT_ON_FILE_GENERIC = "onboarding_signin_phone_not_on_file_generic"
    const val CODE_PROMPT = "onboarding_binding_code_prompt"
    const val CODE_PROMPT_GENERIC = "onboarding_binding_code_prompt_generic"
    const val CODE_INVALID = "onboarding_binding_code_invalid"
    const val CODE_INVALID_GENERIC = "onboarding_binding_code_invalid_generic"
    const val NOTIFICATIONS_WHY = "onboarding_permissions_notifications_why"
    const val NOTIFICATIONS_ALLOW = "onboarding_permissions_allow"
    const val NOTIFICATIONS_DECLINE = "onboarding_permissions_decline"
    const val NOTIFICATIONS_DECLINED = "onboarding_permissions_notifications_declined"
    const val NOTIFICATIONS_DECLINED_GENERIC = "onboarding_permissions_notifications_declined_generic"
    const val AUTOUPDATE_STEP = "onboarding_autoupdate_step"
    const val AUTOUPDATE_ENABLED = "onboarding_autoupdate_enabled"
    const val SIGNIN_AGAIN = "auth_signin_again"
    const val BELOW_MIN_VERSION = "auth_below_min_version"
    const val BELOW_MIN_VERSION_GENERIC = "auth_below_min_version_generic"
    const val SETTINGS_TITLE = "settings_title"
    const val SETTINGS_NOTIFICATIONS = "settings_notifications"
    const val SETTINGS_HELP = "settings_help"
}
