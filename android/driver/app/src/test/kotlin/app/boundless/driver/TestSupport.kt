package app.boundless.driver

import app.boundless.driver.i18n.CatalogDriverStrings
import app.boundless.driver.i18n.DriverStrings
import app.boundless.driver.onboarding.DriverOnboardingScreens
import app.boundless.driver.onboarding.DriverOnboardingViewModel
import app.boundless.driver.onboarding.RecoveryCodeProviding
import app.boundless.rider.onboarding.ManifestProviding
import app.boundless.rider.onboarding.NotificationPermissionRequesting
import app.boundless.rider.onboarding.OnboardingNetworking
import app.boundless.rider.onboarding.OnboardingScreenModel
import app.boundless.rider.onboarding.RiderOnboardingScreens
import uniffi.boundless_ffi_kotlin.BindResult
import uniffi.boundless_ffi_kotlin.SignInResult

/** Shared fixtures for the Driver onboarding tests. (The role-neutral fakes mirror the Rider app's —
 *  a few lines each; AGP testFixtures can't host shared Kotlin on the pinned KGP 2.0.21, so the
 *  catalog resolver is single-sourced in `:rider:shared/main` and these trivial doubles are per-app.) */
object Fixtures {
    /** The admin's personal name, supplied by the manifest in production (ADR-0014). */
    const val ADMIN_NAME = "Sarah"

    /** A sample one-time Recovery Code (ADR-0016 D3 / AC19) — opaque, carries no PII, and (like the
     *  real one) must never be logged (P2). Mirrors the iOS `Fixtures.recoveryCode`. */
    const val RECOVERY_CODE = "4F7K-9Q2M"
}

/** The real merged catalog (shared `:rider:shared` + Driver strings.xml) — the single source of truth
 *  for copy, with no English drift between tests/snapshots and the shipped strings. */
val TestStrings: DriverStrings = CatalogDriverStrings.fromDefaultCatalog()

fun driverScreens(): DriverOnboardingScreens = DriverOnboardingScreens(TestStrings)

/** The shared role-neutral factories the Driver reuses (DriverStrings IS-A RiderStrings). */
fun riderScreens(): RiderOnboardingScreens = RiderOnboardingScreens(TestStrings)

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

class FakeRecovery(override val recoveryCode: String?) : RecoveryCodeProviding

/** Builds a Driver onboarding view model wired to fakes. */
fun makeDriverVM(
    signIn: SignInResult = SignInResult.MEMBER_MATCHED,
    bind: BindResult = BindResult.BOUND,
    granted: Boolean = true,
    hasValidSession: Boolean = false,
    adminName: String? = Fixtures.ADMIN_NAME,
    recoveryCode: String? = Fixtures.RECOVERY_CODE,
): DriverOnboardingViewModel = DriverOnboardingViewModel(
    hasValidSession = hasValidSession,
    networking = FakeNetworking(signIn, bind),
    notifications = FakeNotifications(granted),
    manifest = FakeManifest(adminName),
    recovery = FakeRecovery(recoveryCode),
)

/** Drives a fresh Driver view model through the happy path up to (and stopping at) the core's
 *  Permissions state — where the Driver router first shows the Recovery Code capture interstitial. */
suspend fun advanceToPermissions(vm: DriverOnboardingViewModel) {
    vm.begin()
    vm.submitPhone("5551234567")
    vm.submitCode("123456")
}

/**
 * Every Driver onboarding screen model — the Driver-specific deltas plus the role-neutral steps it
 * reuses from `:rider:shared`. Built with no-op callbacks. Used by the no-signup-route (AC1(b)) and
 * TalkBack-order (AC11) sweeps. Mirrors the iOS `DriverScreenFixtures.allModels` (18 screens).
 */
object DriverScreenFixtures {
    fun allModels(adminName: String? = Fixtures.ADMIN_NAME): List<Pair<String, OnboardingScreenModel>> {
        val d = driverScreens()
        val r = riderScreens()
        return listOf(
            // ── Driver-specific ───────────────────────────────────────────────────────────
            "driverIntro" to d.driverIntro {},
            "reAuthPhoneEntry" to d.reAuthPhoneEntry {},
            "recoveryCodeCapture" to d.recoveryCodeCapture(Fixtures.RECOVERY_CODE) {},
            // ── Reused role-neutral steps (rendered identically to the Rider; the Driver app's own
            //    baselines independently close AC11 for this platform) ───────────────────────
            "phoneEntry" to r.phoneEntry {},
            "phoneEntryOffline" to r.phoneEntry(isOffline = true) {},
            "phoneNotOnFile" to r.phoneNotOnFile(adminName) {},
            "phoneNotOnFileNil" to r.phoneNotOnFile(null) {},
            "deviceBinding" to r.deviceBinding(adminName) {},
            "deviceBindingOffline" to r.deviceBinding(adminName, isOffline = true) {},
            "bindingFailed" to r.bindingFailed(adminName) {},
            "bindingFailedNil" to r.bindingFailed(null) {},
            "permissions" to r.permissions({}, {}),
            "permissionsDeclined" to r.permissionsDeclined(adminName) {},
            "permissionsDeclinedNil" to r.permissionsDeclined(null) {},
            "autoUpdateStep" to r.autoUpdateStep {},
            "autoUpdateEnabled" to r.autoUpdateEnabled {},
            "belowMinVersionNamed" to r.calmHelp(adminName),
            "belowMinVersionGeneric" to r.calmHelp(null),
        )
    }
}
