import Foundation

/// Every user-visible Rider onboarding string, resolved from this module's String Catalog
/// (`Onboarding.xcstrings`, table `Onboarding`). Views and a11y labels call these typed
/// accessors — they never hold a raw key or an English literal (constitution P8). The
/// `{adminName}` placeholders in the catalog are positional `%1$@`; the substitution happens
/// here via `String(format:locale:)`, so the admin's name is *data*, never a hardcoded string.
///
/// Resolution uses `Bundle.module.localizedString(forKey:value:table:)` (not `String(localized:)`,
/// whose `LocalizationValue` wants a compile-time literal) because the keys are identifiers
/// resolved at runtime. Two keys live in the catalog for completeness (AC12) but are rendered by a
/// different surface, so they have no accessor here: the two `admin.onboarding.*` keys (SvelteKit
/// admin UI, spec 001 T15). `auth.signin_again` gained its accessor in T12 (the Driver app reaches
/// this shared kit) — see `signInAgain` below.
public enum L10n {
    private static func resolve(_ key: String) -> String {
        // `value: key` → a missing entry falls back to the visible key (caught by tests / pseudo).
        Bundle.module.localizedString(forKey: key, value: key, table: "Onboarding")
    }

    private static func resolve(_ key: String, _ args: CVarArg...) -> String {
        String(format: resolve(key), locale: .current, arguments: args)
    }

    // ── Affordances (the single large control per step — a11y notes) ─────────────────────
    public static var actionContinue: String { resolve("onboarding.action.continue") }
    public static var actionTryAgain: String { resolve("onboarding.action.try_again") }

    // ── Helper intro (FreshInstall) ──────────────────────────────────────────────────────
    public static var helperIntro: String { resolve("onboarding.helper.intro") }

    // ── Sign-in (PhoneEntry / PhoneNotOnFile) ────────────────────────────────────────────
    public static var phonePrompt: String { resolve("onboarding.signin.phone_prompt") }
    /// `nil` adminName → the name-less fallback (no manifest cached: first-launch race / verify fail),
    /// mirroring `belowMinVersionGeneric` — never a name-slot rendered as an empty string.
    public static func phoneNotOnFile(adminName: String?) -> String {
        guard let adminName else { return resolve("onboarding.signin.phone_not_on_file_generic") }
        return resolve("onboarding.signin.phone_not_on_file", adminName)
    }

    // ── Device binding (Onboarding Code / BindingFailed) ─────────────────────────────────
    public static func codePrompt(adminName: String?) -> String {
        guard let adminName else { return resolve("onboarding.binding.code_prompt_generic") }
        return resolve("onboarding.binding.code_prompt", adminName)
    }
    public static func codeInvalid(adminName: String?) -> String {
        guard let adminName else { return resolve("onboarding.binding.code_invalid_generic") }
        return resolve("onboarding.binding.code_invalid", adminName)
    }

    // ── Permissions (+ declined) ─────────────────────────────────────────────────────────
    public static var notificationsWhy: String { resolve("onboarding.permissions.notifications_why") }
    public static var notificationsAllow: String { resolve("onboarding.permissions.allow") }
    public static var notificationsDecline: String { resolve("onboarding.permissions.decline") }
    public static func notificationsDeclined(adminName: String?) -> String {
        guard let adminName else { return resolve("onboarding.permissions.notifications_declined_generic") }
        return resolve("onboarding.permissions.notifications_declined", adminName)
    }

    // ── Auto-update (step + enabled confirmation) ────────────────────────────────────────
    public static var autoUpdateStep: String { resolve("onboarding.autoupdate.step") }
    public static var autoUpdateEnabled: String { resolve("onboarding.autoupdate.enabled") }

    // ── Re-auth / degradation ────────────────────────────────────────────────────────────
    /// Driver interactive re-auth entry: a Driver whose session expired routes to `PhoneEntry`
    /// (the core's `reauth_state_for(.driver)`, P4) and this leads that screen. Riders never see it
    /// — a lone Rider gets the form-less `NeedsReauthHelp`/`belowMinVersion` calm screen (AC15).
    /// The key lives in this shared catalog; the Driver app (spec 001 T12) renders it via this kit.
    public static var signInAgain: String { resolve("auth.signin_again") }

    /// O4 / `NeedsReauthHelp` calm screen. The name is repeated (never a pronoun) — both `%1$@`.
    public static func belowMinVersion(adminName: String) -> String {
        resolve("auth.below_min_version", adminName)
    }
    /// Name-less fallback when no manifest/admin name is available (offline first launch).
    public static var belowMinVersionGeneric: String { resolve("auth.below_min_version_generic") }

    // ── Rider settings (AC6: NO automatic-updates toggle) ────────────────────────────────
    public static var settingsTitle: String { resolve("settings.title") }
    public static var settingsNotifications: String { resolve("settings.notifications") }
    public static var settingsHelp: String { resolve("settings.help") }
}
