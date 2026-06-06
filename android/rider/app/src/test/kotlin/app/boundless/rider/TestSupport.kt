package app.boundless.rider

import app.boundless.rider.i18n.CatalogRiderStrings
import app.boundless.rider.i18n.RiderStrings
import app.boundless.rider.onboarding.ManifestProviding
import app.boundless.rider.onboarding.NotificationPermissionRequesting
import app.boundless.rider.onboarding.OnboardingNetworking
import app.boundless.rider.onboarding.OnboardingScreenModel
import app.boundless.rider.onboarding.OnboardingViewModel
import app.boundless.rider.onboarding.RiderOnboardingScreens
import uniffi.boundless_ffi_kotlin.BindResult
import uniffi.boundless_ffi_kotlin.Role
import uniffi.boundless_ffi_kotlin.SignInResult

/**
 * Rider onboarding test fixtures. The kit + the catalog resolver ([CatalogRiderStrings]) now live in
 * `:rider:shared`; these fakes are the Rider app's test doubles. (The Driver app has equivalent
 * doubles in its own test source set — a few lines each; AGP testFixtures can't host shared Kotlin on
 * the pinned KGP 2.0.21, so the resolver is single-sourced in `:rider:shared/main` and the trivial
 * fakes are per-app.)
 */
object Fixtures {
    /** The admin's personal name, supplied by the manifest in production (ADR-0014). */
    const val ADMIN_NAME = "Sarah"
}

/** The real catalog (parsed from the shipped `:rider:shared` strings.xml) — the single source of
 *  truth for copy, with no English drift between tests/snapshots and the shipped strings. */
val TestStrings: RiderStrings = CatalogRiderStrings.fromDefaultCatalog()

fun testScreens(): RiderOnboardingScreens = RiderOnboardingScreens(TestStrings)

// MARK: Injected-dependency fakes (the deferred app shell's boundaries)

class FakeNetworking(
    var signInResult: SignInResult = SignInResult.MEMBER_MATCHED,
    var bindResult: BindResult = BindResult.BOUND,
) : OnboardingNetworking {
    override suspend fun signIn(phone: String): SignInResult = signInResult
    override suspend fun bindDevice(code: String): BindResult = bindResult
}

class FakeNotifications(var granted: Boolean) : NotificationPermissionRequesting {
    override suspend fun requestAuthorization(): Boolean = granted
}

class FakeManifest(override val adminName: String?) : ManifestProviding

/** Builds an onboarding view model wired to fakes. */
fun makeOnboardingVM(
    role: Role = Role.RIDER,
    signIn: SignInResult = SignInResult.MEMBER_MATCHED,
    bind: BindResult = BindResult.BOUND,
    granted: Boolean = true,
    hasValidSession: Boolean = false,
    adminName: String? = Fixtures.ADMIN_NAME,
): OnboardingViewModel = OnboardingViewModel(
    role = role,
    hasValidSession = hasValidSession,
    networking = FakeNetworking(signIn, bind),
    notifications = FakeNotifications(granted),
    manifest = FakeManifest(adminName),
)

/** Drives a fresh view model through the happy path up to (and stopping at) the Permissions step. */
suspend fun advanceToPermissions(vm: OnboardingViewModel) {
    vm.begin()
    vm.submitPhone("5551234567")
    vm.submitCode("123456")
}

/**
 * Every Rider onboarding screen model, built with no-op callbacks. Used by the no-signup-route
 * (AC1(b)) and TalkBack-order (AC11) tests to sweep across all screens at once. Mirrors the iOS
 * `ScreenFixtures.allModels`.
 */
object ScreenFixtures {
    fun allModels(adminName: String? = Fixtures.ADMIN_NAME): List<Pair<String, OnboardingScreenModel>> {
        val s = testScreens()
        return listOf(
            "helperIntro" to s.helperIntro {},
            "phoneEntry" to s.phoneEntry {},
            "phoneEntryOffline" to s.phoneEntry(isOffline = true) {},
            "phoneNotOnFile" to s.phoneNotOnFile(adminName) {},
            "deviceBinding" to s.deviceBinding(adminName) {},
            "bindingFailed" to s.bindingFailed(adminName) {},
            "permissions" to s.permissions({}, {}),
            "permissionsDeclined" to s.permissionsDeclined(adminName) {},
            "autoUpdateStep" to s.autoUpdateStep {},
            "autoUpdateEnabled" to s.autoUpdateEnabled {},
            "belowMinVersionNamed" to s.calmHelp(adminName),
            "belowMinVersionGeneric" to s.calmHelp(null),
            // Name-less variants (no manifest cached): the four name-bearing screens must render a
            // generic fallback, never an empty name slot (reviewer T11 confirmed finding).
            "phoneNotOnFileNil" to s.phoneNotOnFile(null) {},
            "deviceBindingNil" to s.deviceBinding(null) {},
            "bindingFailedNil" to s.bindingFailed(null) {},
            "permissionsDeclinedNil" to s.permissionsDeclined(null) {},
        )
    }
}
