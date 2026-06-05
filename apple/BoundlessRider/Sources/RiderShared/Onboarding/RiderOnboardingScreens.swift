import SwiftUI

/// Factories that build each Rider onboarding screen's `OnboardingScreenModel` from data + the
/// callbacks the router wires to the view model. Every screen leads with its primary copy as a
/// VoiceOver header (a11y bar). Strings come only from `L10n` (the catalog) — never literals (P8).
///
/// The screens RENDER the `core::auth` state machine's states (via `OnboardingState` from
/// `BoundlessKit`); they never decide transitions (P4). The terminal calm screens
/// (`belowMinVersion`, `needsReauthHelp`) and the permission-declined acknowledgement carry **no**
/// field and **no** advancing control beyond what the spec allows — never a sign-in form (AC15),
/// never an "Update Now" CTA (AC8).
public enum RiderOnboardingScreens {
    // MARK: FreshInstall → helper intro

    public static func helperIntro(onContinue: @escaping () -> Void) -> OnboardingScreenModel {
        OnboardingScreenModel(
            elements: [.heading(L10n.helperIntro)],
            actions: [ScreenAction(label: L10n.actionContinue, perform: onContinue)]
        )
    }

    // MARK: PhoneEntry (+ PhoneNotOnFile banner, + Offline overlay)

    public static func phoneEntry(
        text: Binding<String>,
        isOffline: Bool = false,
        onContinue: @escaping () -> Void
    ) -> OnboardingScreenModel {
        OnboardingScreenModel(
            elements: [.heading(L10n.phonePrompt)],
            field: FieldModel(label: L10n.phonePrompt, kind: .phone, text: text),
            actions: [ScreenAction(label: L10n.actionContinue, perform: onContinue)],
            isOffline: isOffline
        )
    }

    public static func phoneNotOnFile(
        text: Binding<String>,
        adminName: String?,
        onTryAgain: @escaping () -> Void
    ) -> OnboardingScreenModel {
        OnboardingScreenModel(
            elements: [
                .heading(L10n.phonePrompt),
                .banner(L10n.phoneNotOnFile(adminName: adminName)),
            ],
            field: FieldModel(label: L10n.phonePrompt, kind: .phone, text: text),
            actions: [ScreenAction(label: L10n.actionTryAgain, perform: onTryAgain)]
        )
    }

    // MARK: DeviceBinding (+ BindingFailed banner, + Offline overlay)

    public static func deviceBinding(
        text: Binding<String>,
        adminName: String?,
        isOffline: Bool = false,
        onContinue: @escaping () -> Void
    ) -> OnboardingScreenModel {
        OnboardingScreenModel(
            elements: [.heading(L10n.codePrompt(adminName: adminName))],
            field: FieldModel(label: L10n.codePrompt(adminName: adminName), kind: .code, text: text),
            actions: [ScreenAction(label: L10n.actionContinue, perform: onContinue)],
            isOffline: isOffline
        )
    }

    public static func bindingFailed(
        text: Binding<String>,
        adminName: String?,
        onTryAgain: @escaping () -> Void
    ) -> OnboardingScreenModel {
        OnboardingScreenModel(
            elements: [
                .heading(L10n.codePrompt(adminName: adminName)),
                .banner(L10n.codeInvalid(adminName: adminName)),
            ],
            field: FieldModel(label: L10n.codePrompt(adminName: adminName), kind: .code, text: text),
            actions: [ScreenAction(label: L10n.actionTryAgain, perform: onTryAgain)]
        )
    }

    // MARK: Permissions (+ declined acknowledgement)

    public static func permissions(
        onAllow: @escaping () -> Void,
        onDecline: @escaping () -> Void
    ) -> OnboardingScreenModel {
        OnboardingScreenModel(
            elements: [.heading(L10n.notificationsWhy)],
            actions: [
                ScreenAction(label: L10n.notificationsAllow, emphasis: .primary, perform: onAllow),
                ScreenAction(label: L10n.notificationsDecline, emphasis: .secondary, perform: onDecline),
            ]
        )
    }

    /// Shown only on decline — calm, no scolding (AC14). Advances to the auto-update step.
    public static func permissionsDeclined(
        adminName: String?,
        onContinue: @escaping () -> Void
    ) -> OnboardingScreenModel {
        OnboardingScreenModel(
            elements: [.heading(L10n.notificationsDeclined(adminName: adminName))],
            actions: [ScreenAction(label: L10n.actionContinue, perform: onContinue)]
        )
    }

    // MARK: AutoUpdateStep (step + "auto-update enabled" confirmation)

    public static func autoUpdateStep(onContinue: @escaping () -> Void) -> OnboardingScreenModel {
        OnboardingScreenModel(
            elements: [.heading(L10n.autoUpdateStep)],
            actions: [ScreenAction(label: L10n.actionContinue, perform: onContinue)]
        )
    }

    /// The screen AC5 asserts ("auto-update enabled"). The confirmation is a completed state, not a
    /// button. A `Continue` advances to the (silent) completion.
    public static func autoUpdateEnabled(onContinue: @escaping () -> Void) -> OnboardingScreenModel {
        OnboardingScreenModel(
            elements: [.confirmation(L10n.autoUpdateEnabled)],
            actions: [ScreenAction(label: L10n.actionContinue, perform: onContinue)]
        )
    }

    // MARK: Terminal calm screens — BelowMinVersion / NeedsReauthHelp (no CTA, no form)

    /// The calm degradation / re-auth-help screen (O4 / AC8 / AC15). Uses the admin's name when a
    /// manifest is cached, else the name-less fallback. **No** advancing control, **no** field —
    /// never an "Update Now" CTA, never a sign-in form (P10).
    public static func calmHelp(adminName: String?) -> OnboardingScreenModel {
        let message = adminName.map(L10n.belowMinVersion(adminName:)) ?? L10n.belowMinVersionGeneric
        return OnboardingScreenModel(elements: [.heading(message)])
    }
}
