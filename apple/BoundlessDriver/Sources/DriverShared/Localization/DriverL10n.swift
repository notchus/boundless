import Foundation

/// Driver-only onboarding strings, resolved from this module's String Catalog
/// (`DriverOnboarding.xcstrings`, table `DriverOnboarding`). Views and a11y labels call these typed
/// accessors — they never hold a raw key or an English literal (constitution P8).
///
/// The Driver app resolves *shared* onboarding copy (phone prompt, code prompt, permissions,
/// auto-update, `auth.signin_again`, the degradation screens) through `RiderShared.L10n`, which reads
/// from `RiderShared`'s own bundle. Only the genuinely Driver-specific keys live here: the
/// self-onboard intro and the Recovery Code capture screen (none take `{adminName}`, so there is no
/// positional-argument substitution — the resolver is the simple form).
public enum DriverL10n {
    private static func resolve(_ key: String) -> String {
        // `value: key` → a missing entry falls back to the visible key (caught by tests / pseudo).
        Bundle.module.localizedString(forKey: key, value: key, table: "DriverOnboarding")
    }

    /// Driver self-onboarding intro (`FreshInstall`). Self-directed — distinct from the Rider's
    /// helper-facing `onboarding.helper.intro`.
    public static var driverIntro: String { resolve("onboarding.driver.intro") }

    // ── Recovery Code capture (Driver only; ADR-0016 D3 / AC19) ──────────────────────────
    public static var recoveryTitle: String { resolve("onboarding.recovery.title") }
    public static var recoveryExplanation: String { resolve("onboarding.recovery.explanation") }
    public static var recoverySaved: String { resolve("onboarding.recovery.saved") }
}
