package app.boundless.rider.onboarding

import uniffi.boundless_ffi_kotlin.BindResult
import uniffi.boundless_ffi_kotlin.SignInResult

/**
 * The injected side-effect boundaries the onboarding flow depends on. The real implementations are
 * the deferred app shell (T13-shell): the OpenAPI Kotlin HTTP client, the `NotificationManager`
 * permission flow, and the signed-KV-manifest fetch/verify. The flow depends only on these
 * interfaces, so it is deterministic in tests (fakes) and the view model never decides outcomes —
 * it feeds the interpreted result straight into the core state machine (P4).
 *
 * Android twins of `RiderShared`'s `OnboardingNetworking` / `NotificationPermissionRequesting` /
 * `ManifestProviding` protocols.
 */
interface OnboardingNetworking {
    suspend fun signIn(phone: String): SignInResult
    suspend fun bindDevice(code: String): BindResult
}

interface NotificationPermissionRequesting {
    /** Returns whether notification permission was granted. */
    suspend fun requestAuthorization(): Boolean
}

interface ManifestProviding {
    /** The per-Group admin name from the signed KV manifest (ADR-0014), read from cache at launch.
     *  `null` → the name-less fallback copy is used. */
    val adminName: String?
}
