package app.boundless.driver

import app.boundless.rider.onboarding.BodyElement
import app.boundless.rider.onboarding.OnboardingScreenModel
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test
import uniffi.boundless_ffi_kotlin.OnboardingEvent
import uniffi.boundless_ffi_kotlin.OnboardingState
import uniffi.boundless_ffi_kotlin.onEvent

/**
 * AC1(b) — the Driver onboarding entry flow exposes no "sign up" / "create account" / "request access"
 * route (asserts I11). Verified against the state machine (which has no signup state) and against every
 * rendered Driver screen's copy + affordances. The Driver's entry AND its re-auth are sign-IN (to an
 * existing admin-issued account), never sign-UP. Twin of `NoDriverSignupRouteTests.swift`.
 */
class NoDriverSignupRouteTest {
    private val forbidden = listOf(
        "sign up", "signup", "sign-up",
        "create account", "create an account", "create your account",
        "request access", "request an account",
        "register", "join now", "get started",
    )

    @Test
    fun compose_driver_no_signup_route() {
        for ((name, model) in DriverScreenFixtures.allModels()) {
            val strings = textContent(model) + model.actionLabels
            for (text in strings) {
                val lower = text.lowercase()
                for (term in forbidden) {
                    assertFalse(
                        "Screen '$name' exposes a signup-like affordance/copy: '$text' (AC1(b)).",
                        lower.contains(term),
                    )
                }
            }
        }
    }

    /** The entry from a fresh install is sign-IN (to an existing admin-issued account), never sign-UP:
     *  the only forward transition from FRESH_INSTALL leads to PHONE_ENTRY (I11). The Driver's re-auth
     *  is also PHONE_ENTRY (`reauthStateFor(DRIVER)`), i.e. sign-in — see DriverReauthRouteTest. */
    @Test
    fun entryFromFreshInstallIsSignIn() {
        assertEquals(
            OnboardingState.PHONE_ENTRY,
            onEvent(OnboardingState.FRESH_INSTALL, OnboardingEvent.BeginSignIn),
        )
    }

    /** A Driver with an invalidated session re-auths via the interactive PhoneEntry form — never
     *  account creation. The Recovery Code capture is a value to save, not a signup affordance: it
     *  has a confirm action only, no field. */
    @Test
    fun reAuthAndRecoveryAreNotSignup() {
        assertFalse(driverScreens().recoveryCodeCapture(Fixtures.RECOVERY_CODE) {}.hasInputAffordance)
        // The re-auth screen IS a sign-in form (a phone field), the opposite of account creation.
        assertEquals(true, driverScreens().reAuthPhoneEntry {}.hasInputAffordance)
    }

    private fun textContent(model: OnboardingScreenModel): List<String> =
        model.elements.map { element ->
            when (element) {
                is BodyElement.Heading -> element.text
                is BodyElement.Paragraph -> element.text
                is BodyElement.Banner -> element.text
                is BodyElement.Confirmation -> element.text
                is BodyElement.Code -> element.text
            }
        }
}
