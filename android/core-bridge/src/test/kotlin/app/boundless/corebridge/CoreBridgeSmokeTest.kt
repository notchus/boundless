package app.boundless.corebridge

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.boundless_ffi_kotlin.BindResult
import uniffi.boundless_ffi_kotlin.LaunchDecision
import uniffi.boundless_ffi_kotlin.OnboardingEvent
import uniffi.boundless_ffi_kotlin.OnboardingState
import uniffi.boundless_ffi_kotlin.Role
import uniffi.boundless_ffi_kotlin.SignInResult
import uniffi.boundless_ffi_kotlin.allowsOfflineOverlay
import uniffi.boundless_ffi_kotlin.isTerminal
import uniffi.boundless_ffi_kotlin.launch
import uniffi.boundless_ffi_kotlin.onEvent
import uniffi.boundless_ffi_kotlin.reauthStateFor
import uniffi.boundless_ffi_kotlin.shouldFlagNotificationsOff

/**
 * The Android twin of the BoundlessKit on-simulator smoke test: it drives the SAME onboarding
 * graph through the SAME core, but via Rust → UniFFI → **Kotlin/JNA** on the host JVM (no
 * emulator — the generated bindings are pure JVM). Green here means the whole bring-up works:
 * `core/ffi-kotlin` → cargo cdylib → uniffi-bindgen Kotlin → JNA load → core transition table (P4).
 *
 * The host cdylib is loaded from `core/target/release` via the `jna.library.path` set in this
 * module's build.gradle.kts; `scripts/build-corebridge.sh` builds it alongside the bindings.
 */
class CoreBridgeSmokeTest {
    @Test
    fun ffiDrivesTheCoreOnboardingGraph() {
        assertEquals(LaunchDecision.ONBOARD, launch(false))
        assertEquals(LaunchDecision.RESUME, launch(true))

        // Happy path: fresh install → … → complete (terminal).
        var s = OnboardingState.FRESH_INSTALL
        s = onEvent(s, OnboardingEvent.BeginSignIn)
        assertEquals(OnboardingState.PHONE_ENTRY, s)
        s = onEvent(s, OnboardingEvent.SignIn(SignInResult.MEMBER_MATCHED))
        assertEquals(OnboardingState.DEVICE_BINDING, s)
        s = onEvent(s, OnboardingEvent.Bind(BindResult.BOUND))
        assertEquals(OnboardingState.PERMISSIONS, s)
        s = onEvent(s, OnboardingEvent.PermissionDecision(false))
        assertEquals(OnboardingState.AUTO_UPDATE_STEP, s)
        s = onEvent(s, OnboardingEvent.AutoUpdateConfirmed)
        assertEquals(OnboardingState.COMPLETE, s)
        assertTrue(isTerminal(s))

        // Cross-cutting routing (AC15/AC14 + the offline-overlay rule).
        assertEquals(OnboardingState.NEEDS_REAUTH_HELP, reauthStateFor(Role.RIDER))
        assertEquals(OnboardingState.PHONE_ENTRY, reauthStateFor(Role.DRIVER))
        assertTrue(allowsOfflineOverlay(OnboardingState.PHONE_ENTRY))
        assertFalse(allowsOfflineOverlay(OnboardingState.PERMISSIONS))
        assertTrue(shouldFlagNotificationsOff(false))
        assertFalse(shouldFlagNotificationsOff(true))
    }
}
