package app.boundless.driver.onboarding

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import app.boundless.rider.onboarding.ManifestProviding
import app.boundless.rider.onboarding.NotificationPermissionRequesting
import app.boundless.rider.onboarding.OnboardingNetworking
import app.boundless.rider.onboarding.OnboardingViewModel
import uniffi.boundless_ffi_kotlin.OnboardingState
import uniffi.boundless_ffi_kotlin.Role

/**
 * Drives the **Driver** onboarding flow. It composes the shared [OnboardingViewModel] with
 * `role = DRIVER` — so every transition is still the core's decision via `:core-bridge` (P4 /
 * ADR-0001 / ADR-0022), never re-implemented here — and adds only the two Driver-specific concerns the
 * shared model doesn't carry: the captured Recovery Code (for the AC19 capture screen) and the fact
 * that an invalidated Driver session routes to interactive re-auth at `PhoneEntry` (vs the Rider's
 * calm `NeedsReauthHelp`). Side effects stay behind the injected interfaces, so the flow is
 * deterministic in tests. A plain class (no `androidx.lifecycle.ViewModel`, no Hilt — deps via
 * constructor, mirroring `DriverShared.DriverOnboardingViewModel`); state is Compose-observable
 * (the wrapped [OnboardingViewModel]'s `mutableStateOf`, plus [reauthRequested]) so the router recomposes.
 */
class DriverOnboardingViewModel(
    hasValidSession: Boolean,
    networking: OnboardingNetworking,
    notifications: NotificationPermissionRequesting,
    manifest: ManifestProviding,
    private val recovery: RecoveryCodeProviding,
) {
    private val core = OnboardingViewModel(
        role = Role.DRIVER,
        hasValidSession = hasValidSession,
        networking = networking,
        notifications = notifications,
        manifest = manifest,
    )

    /** Set when an invalidated session routed this Driver back to `PhoneEntry`, so the router shows
     *  the re-auth variant (`auth.signin_again`) rather than the fresh sign-in. The *target state* is
     *  still the core's decision (`reauthStateFor(DRIVER)`); this only records that it happened. */
    var reauthRequested: Boolean by mutableStateOf(false)
        private set

    // ── Shared state (forwarded from the core view model) ─────────────────────────────────
    val state: OnboardingState get() = core.state
    val adminName: String? get() = core.adminName
    val notificationsFlaggedOff: Boolean get() = core.notificationsFlaggedOff

    /** The one-time Recovery Code to display on the capture screen, or `null` if none is available
     *  (the router then skips the capture — never an empty-code screen, never a block). */
    val recoveryCode: String? get() = recovery.recoveryCode

    // ── Events — each delegates the transition to the core (never decided here, P4) ─────────
    fun begin() = core.begin()
    suspend fun submitPhone(phone: String) = core.submitPhone(phone)
    suspend fun submitCode(code: String) = core.submitCode(code)
    suspend fun decideNotifications(allow: Boolean) = core.decideNotifications(allow)
    fun confirmAutoUpdate() = core.confirmAutoUpdate()
    fun belowMinVersionDetected() = core.belowMinVersionDetected()

    /** A previously-valid Driver session was invalidated mid-life. The core routes a Driver to
     *  `PhoneEntry` for interactive re-auth (AC15/AC18); we record that so the router leads that
     *  screen with `auth.signin_again`. */
    fun sessionInvalidated() {
        reauthRequested = true
        core.sessionInvalidated()
    }
}
