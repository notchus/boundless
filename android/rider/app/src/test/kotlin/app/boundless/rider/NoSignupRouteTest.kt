package app.boundless.rider

import app.boundless.rider.onboarding.BodyElement
import app.boundless.rider.onboarding.OnboardingScreenModel
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test
import uniffi.boundless_ffi_kotlin.OnboardingEvent
import uniffi.boundless_ffi_kotlin.OnboardingState
import uniffi.boundless_ffi_kotlin.onEvent

/**
 * AC1(b) — the Rider onboarding entry flow exposes no "sign up" / "create account" / "request access"
 * route (asserts I11). Verified against the state machine (which has no signup state) and against
 * every rendered screen's copy + affordances. Twin of `NoSignupRouteTests.swift`.
 */
class NoSignupRouteTest {
    private val forbidden = listOf(
        "sign up", "signup", "sign-up",
        "create account", "create an account", "create your account",
        "request access", "request an account",
        "register", "join now", "get started",
    )

    @Test
    fun compose_onboarding_no_signup_route() {
        for ((name, model) in ScreenFixtures.allModels()) {
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
     *  the only forward transition from FRESH_INSTALL leads to PHONE_ENTRY (I11). */
    @Test
    fun entryFromFreshInstallIsSignIn() {
        assertEquals(OnboardingState.PHONE_ENTRY, onEvent(OnboardingState.FRESH_INSTALL, OnboardingEvent.BeginSignIn))
    }

    /** A lone Rider with an invalidated session cannot self-serve into account creation — the calm
     *  terminal screen has no input at all (reinforces AC1(b) alongside AC15). */
    @Test
    fun terminalScreensHaveNoInput() {
        assertFalse(testScreens().calmHelp(Fixtures.ADMIN_NAME).hasInputAffordance)
        assertFalse(testScreens().calmHelp(null).hasInputAffordance)
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
