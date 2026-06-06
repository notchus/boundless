package app.boundless.driver.onboarding

import app.boundless.driver.i18n.DriverStrings
import app.boundless.rider.onboarding.BodyElement
import app.boundless.rider.onboarding.FieldModel
import app.boundless.rider.onboarding.OnboardingScreenModel
import app.boundless.rider.onboarding.ScreenAction

/**
 * The Driver-specific onboarding screens — the deltas over the shared `:rider:shared` kit. The
 * role-neutral steps (phone entry, device binding, permissions, auto-update, the calm degradation
 * screen) are reused verbatim from [app.boundless.rider.onboarding.RiderOnboardingScreens] (P4 / no
 * duplication); only these three are genuinely Driver-only. Every screen is an [OnboardingScreenModel],
 * so it renders through the single shared `OnboardingScreenView` and derives its a11y reading order the
 * same way (no drift). Strings come only from [DriverStrings] (which provides both the shared accessors
 * and the Driver keys) — never literals (P8). The Android twin of `DriverShared.DriverOnboardingScreens`.
 */
class DriverOnboardingScreens(private val strings: DriverStrings) {

    /** `FreshInstall` for a Driver, who runs setup themselves — self-directed copy, not the Rider's
     *  helper-facing "…together". */
    fun driverIntro(onContinue: () -> Unit) = OnboardingScreenModel(
        elements = listOf(BodyElement.Heading(strings.driverIntro)),
        actions = listOf(ScreenAction(strings.actionContinue, onClick = onContinue)),
    )

    /** Interactive re-auth `PhoneEntry` (AC15 Driver branch): a Driver whose session expired is routed
     *  here by the core (`reauthStateFor(DRIVER) == PHONE_ENTRY`, P4) — unlike a Rider, who gets the
     *  form-less `NeedsReauthHelp`. Leads with `auth.signin_again` and *is* a sign-in form, by design. */
    fun reAuthPhoneEntry(onContinue: () -> Unit) = OnboardingScreenModel(
        elements = listOf(BodyElement.Heading(strings.signInAgain)),
        field = FieldModel(strings.phonePrompt, FieldModel.Kind.PHONE),
        actions = listOf(ScreenAction(strings.actionContinue, onClick = onContinue)),
    )

    /** The Driver's one-time **Recovery Code capture** screen (ADR-0016 D3 / AC19). Shown once, right
     *  after the device is bound and the session issued, so the Driver can self-serve a future device
     *  replacement. The [code] is *data* from the bind response (never a catalog string); it renders
     *  via the [BodyElement.Code] element (prominent, monospaced, selectable) and is never logged (P2).
     *  Not a form — a single confirm action ("I've saved it"), no field. */
    fun recoveryCodeCapture(code: String, onContinue: () -> Unit) = OnboardingScreenModel(
        elements = listOf(
            BodyElement.Heading(strings.recoveryTitle),
            BodyElement.Paragraph(strings.recoveryExplanation),
            BodyElement.Code(code),
        ),
        actions = listOf(ScreenAction(strings.recoverySaved, onClick = onContinue)),
    )
}
