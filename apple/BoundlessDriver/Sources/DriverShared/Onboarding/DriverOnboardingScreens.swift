import RiderShared
import SwiftUI

/// The Driver-specific onboarding screens — the deltas over the shared `RiderShared` kit. The
/// role-neutral steps (phone entry, device binding, permissions, auto-update, the calm degradation
/// screen) are reused verbatim from `RiderOnboardingScreens` (P4 / no duplication); only these three
/// are genuinely Driver-only. Every screen is an `OnboardingScreenModel`, so it renders through the
/// single shared `OnboardingScreenView` and derives its a11y reading order the same way (no drift).
/// Strings come only from the catalogs (`L10n` / `DriverL10n`) — never literals (P8).
public enum DriverOnboardingScreens {
    /// `FreshInstall` for a Driver, who runs setup themselves — self-directed copy, not the Rider's
    /// helper-facing "…together".
    public static func driverIntro(onContinue: @escaping () -> Void) -> OnboardingScreenModel {
        OnboardingScreenModel(
            elements: [.heading(DriverL10n.driverIntro)],
            actions: [ScreenAction(label: L10n.actionContinue, perform: onContinue)]
        )
    }

    /// Interactive re-auth `PhoneEntry` (AC15 Driver branch): a Driver whose session expired is routed
    /// here by the core (`reauth_state_for(.driver) == .phoneEntry`, P4) — unlike a Rider, who gets the
    /// form-less `NeedsReauthHelp`. Leads with `auth.signin_again` and *is* a sign-in form, by design.
    public static func reAuthPhoneEntry(
        text: Binding<String>,
        onContinue: @escaping () -> Void
    ) -> OnboardingScreenModel {
        OnboardingScreenModel(
            elements: [.heading(L10n.signInAgain)],
            field: FieldModel(label: L10n.phonePrompt, kind: .phone, text: text),
            actions: [ScreenAction(label: L10n.actionContinue, perform: onContinue)]
        )
    }

    /// The Driver's one-time **Recovery Code capture** screen (ADR-0016 D3 / AC19). Shown once, right
    /// after the device is bound and the session issued, so the Driver can self-serve a future device
    /// replacement. The `code` is *data* from the bind response (never a catalog string); it renders
    /// via the `.code` element (prominent, monospaced, selectable) and is never logged (P2).
    public static func recoveryCodeCapture(
        code: String,
        onContinue: @escaping () -> Void
    ) -> OnboardingScreenModel {
        OnboardingScreenModel(
            elements: [
                .heading(DriverL10n.recoveryTitle),
                .paragraph(DriverL10n.recoveryExplanation),
                .code(code),
            ],
            actions: [ScreenAction(label: DriverL10n.recoverySaved, perform: onContinue)]
        )
    }
}
