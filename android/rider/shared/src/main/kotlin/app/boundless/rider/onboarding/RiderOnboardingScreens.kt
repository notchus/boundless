package app.boundless.rider.onboarding

import app.boundless.rider.i18n.RiderStrings

/**
 * Factories that build each Rider onboarding screen's [OnboardingScreenModel] from data + the
 * callbacks the router wires to the view model. Every screen leads with its primary copy as a
 * TalkBack heading (a11y bar). Strings come only from [RiderStrings] (the catalog) — never literals
 * (P8). The screens RENDER the `core::auth` state machine's states; they never decide transitions
 * (P4). The terminal calm screens (below-min / needs-reauth) and the permission-declined
 * acknowledgement carry **no** field and **no** advancing control beyond what the spec allows —
 * never a sign-in form (AC15), never an "Update Now" CTA (AC8).
 *
 * The Android twin of `RiderShared.RiderOnboardingScreens`.
 */
class RiderOnboardingScreens(private val strings: RiderStrings) {

    // FreshInstall → helper intro
    fun helperIntro(onContinue: () -> Unit) = OnboardingScreenModel(
        elements = listOf(BodyElement.Heading(strings.helperIntro)),
        actions = listOf(ScreenAction(strings.actionContinue, onClick = onContinue)),
    )

    // PhoneEntry (+ PhoneNotOnFile banner, + Offline overlay)
    fun phoneEntry(isOffline: Boolean = false, onContinue: () -> Unit) = OnboardingScreenModel(
        elements = listOf(BodyElement.Heading(strings.phonePrompt)),
        field = FieldModel(strings.phonePrompt, FieldModel.Kind.PHONE),
        actions = listOf(ScreenAction(strings.actionContinue, onClick = onContinue)),
        isOffline = isOffline,
    )

    fun phoneNotOnFile(adminName: String?, onTryAgain: () -> Unit) = OnboardingScreenModel(
        elements = listOf(
            BodyElement.Heading(strings.phonePrompt),
            BodyElement.Banner(strings.phoneNotOnFile(adminName)),
        ),
        field = FieldModel(strings.phonePrompt, FieldModel.Kind.PHONE),
        actions = listOf(ScreenAction(strings.actionTryAgain, onClick = onTryAgain)),
    )

    // DeviceBinding (+ BindingFailed banner, + Offline overlay)
    fun deviceBinding(adminName: String?, isOffline: Boolean = false, onContinue: () -> Unit) =
        OnboardingScreenModel(
            elements = listOf(BodyElement.Heading(strings.codePrompt(adminName))),
            field = FieldModel(strings.codePrompt(adminName), FieldModel.Kind.CODE),
            actions = listOf(ScreenAction(strings.actionContinue, onClick = onContinue)),
            isOffline = isOffline,
        )

    fun bindingFailed(adminName: String?, onTryAgain: () -> Unit) = OnboardingScreenModel(
        elements = listOf(
            BodyElement.Heading(strings.codePrompt(adminName)),
            BodyElement.Banner(strings.codeInvalid(adminName)),
        ),
        field = FieldModel(strings.codePrompt(adminName), FieldModel.Kind.CODE),
        actions = listOf(ScreenAction(strings.actionTryAgain, onClick = onTryAgain)),
    )

    // Permissions (+ declined acknowledgement)
    fun permissions(onAllow: () -> Unit, onDecline: () -> Unit) = OnboardingScreenModel(
        elements = listOf(BodyElement.Heading(strings.notificationsWhy)),
        actions = listOf(
            ScreenAction(strings.notificationsAllow, ScreenAction.Emphasis.PRIMARY, onAllow),
            ScreenAction(strings.notificationsDecline, ScreenAction.Emphasis.SECONDARY, onDecline),
        ),
    )

    /** Shown only on decline — calm, no scolding (AC14). Advances to the auto-update step. */
    fun permissionsDeclined(adminName: String?, onContinue: () -> Unit) = OnboardingScreenModel(
        elements = listOf(BodyElement.Heading(strings.notificationsDeclined(adminName))),
        actions = listOf(ScreenAction(strings.actionContinue, onClick = onContinue)),
    )

    // AutoUpdateStep (step + "auto-update enabled" confirmation)
    fun autoUpdateStep(onContinue: () -> Unit) = OnboardingScreenModel(
        elements = listOf(BodyElement.Heading(strings.autoUpdateStep)),
        actions = listOf(ScreenAction(strings.actionContinue, onClick = onContinue)),
    )

    /** The screen AC5 asserts ("auto-update enabled"). The confirmation is a completed state, not a
     *  button. A Continue advances to the (silent) completion. */
    fun autoUpdateEnabled(onContinue: () -> Unit) = OnboardingScreenModel(
        elements = listOf(BodyElement.Confirmation(strings.autoUpdateEnabled)),
        actions = listOf(ScreenAction(strings.actionContinue, onClick = onContinue)),
    )

    /** The calm degradation / re-auth-help screen (O4 / AC8 / AC15). Uses the admin's name when a
     *  manifest is cached, else the name-less fallback. **No** advancing control, **no** field —
     *  never an "Update Now" CTA, never a sign-in form (P10). Serves both `BELOW_MIN_VERSION` and
     *  (for a Rider) `NEEDS_REAUTH_HELP`. */
    fun calmHelp(adminName: String?) = OnboardingScreenModel(
        elements = listOf(
            BodyElement.Heading(
                if (adminName != null) strings.belowMinVersion(adminName) else strings.belowMinVersionGeneric,
            ),
        ),
    )
}
