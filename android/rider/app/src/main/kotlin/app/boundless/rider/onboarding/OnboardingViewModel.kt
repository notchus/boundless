package app.boundless.rider.onboarding

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import uniffi.boundless_ffi_kotlin.LaunchDecision
import uniffi.boundless_ffi_kotlin.OnboardingEvent
import uniffi.boundless_ffi_kotlin.OnboardingState
import uniffi.boundless_ffi_kotlin.Role
import uniffi.boundless_ffi_kotlin.launch
import uniffi.boundless_ffi_kotlin.onEvent
import uniffi.boundless_ffi_kotlin.shouldFlagNotificationsOff

/**
 * Drives the Rider onboarding flow. Holds the current [OnboardingState] and applies events through
 * the `core::auth` state machine exported by `:core-bridge` — **every** transition is the core's
 * decision (`onEvent`), never re-implemented here (constitution P4 / ADR-0001 / ADR-0022). Side
 * effects (network, OS permission, manifest) are injected, so the whole flow is deterministic in
 * tests. A plain class (no `androidx.lifecycle.ViewModel`, no Hilt — deps via constructor, mirroring
 * `RiderShared.OnboardingViewModel`); state is Compose [mutableStateOf] so the router recomposes.
 */
class OnboardingViewModel(
    val role: Role,
    hasValidSession: Boolean,
    private val networking: OnboardingNetworking,
    private val notifications: NotificationPermissionRequesting,
    private val manifest: ManifestProviding,
) {
    // Launch routing is the core's decision (ADR-0016 D2): a live session resumes straight to the
    // primary surface (modelled here as COMPLETE); otherwise onboarding begins.
    var state: OnboardingState by mutableStateOf(
        if (launch(hasValidSession) == LaunchDecision.RESUME) OnboardingState.COMPLETE
        else OnboardingState.FRESH_INSTALL,
    )
        private set

    /** Set when a declined (or unavailable) notification permission must be recorded as a non-PII
     *  admin flag (AC14). The CORE decides this (`shouldFlagNotificationsOff`), not the UI (P4);
     *  actually *sending* the flag to the server is the deferred shell. */
    var notificationsFlaggedOff: Boolean by mutableStateOf(false)
        private set

    /** The admin's name for `{adminName}` copy, or `null` → the name-less fallback (offline launch). */
    val adminName: String? get() = manifest.adminName

    // ── Events — each delegates the transition to the core (never decided here, P4) ──────────

    fun begin() = applyEvent(OnboardingEvent.BeginSignIn)

    suspend fun submitPhone(phone: String) {
        // From the PhoneNotOnFile banner, return to PhoneEntry first, then re-evaluate the lookup.
        if (state == OnboardingState.PHONE_NOT_ON_FILE) applyEvent(OnboardingEvent.RetryPhoneEntry)
        applyEvent(OnboardingEvent.SignIn(networking.signIn(phone)))
    }

    suspend fun submitCode(code: String) {
        if (state == OnboardingState.BINDING_FAILED) applyEvent(OnboardingEvent.RetryBinding)
        applyEvent(OnboardingEvent.Bind(networking.bindDevice(code)))
    }

    /** `allow == true` requests OS permission (showing the system prompt); `allow == false`
     *  ("Not now") skips it. Either way the flow advances and never scolds (AC14). */
    suspend fun decideNotifications(allow: Boolean) {
        val granted = if (allow) notifications.requestAuthorization() else false
        if (shouldFlagNotificationsOff(granted)) notificationsFlaggedOff = true
        applyEvent(OnboardingEvent.PermissionDecision(granted))
    }

    fun confirmAutoUpdate() = applyEvent(OnboardingEvent.AutoUpdateConfirmed)

    /** A previously-valid session was invalidated mid-life (admin revoke / new-device / deletion).
     *  Routes per the core: a Rider → calm help; a Driver → interactive re-auth (AC15/AC18). */
    fun sessionInvalidated() = applyEvent(OnboardingEvent.SessionInvalidated(role))

    /** A below-`client_min_version` handshake was detected (O4) — reachable from any state. */
    fun belowMinVersionDetected() = applyEvent(OnboardingEvent.BelowMinVersionDetected)

    private fun applyEvent(event: OnboardingEvent) {
        state = onEvent(state, event)
    }
}
