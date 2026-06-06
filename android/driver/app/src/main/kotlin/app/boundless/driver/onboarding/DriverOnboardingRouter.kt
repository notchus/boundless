package app.boundless.driver.onboarding

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import app.boundless.driver.i18n.DriverStrings
import app.boundless.rider.onboarding.OnboardingScreenView
import app.boundless.rider.onboarding.PrimarySurfacePlaceholder
import app.boundless.rider.onboarding.RiderOnboardingScreens
import kotlinx.coroutines.launch
import uniffi.boundless_ffi_kotlin.OnboardingState

/**
 * Maps the current [OnboardingState] to the screen the Driver sees, wiring each affordance to the view
 * model. Like the Rider router, it owns only *view* state (the field text and the in-step
 * acknowledgements); the authoritative [OnboardingState] lives in the view model and is advanced solely
 * by the core (P4). The role-neutral steps reuse [RiderOnboardingScreens] verbatim (the shared kit);
 * the Driver-only screens come from [DriverOnboardingScreens]. Completion is **silent** — `COMPLETE`
 * routes to the primary surface with no "all set" screen (voice-and-tone). The Android twin of
 * `DriverShared.DriverOnboardingRouter`.
 *
 * The composition root (MainActivity, deferred to T14-shell) supplies the [strings] resolver and
 * constructs the [viewModel] with the real conformers (incl. the real `RecoveryCodeProviding`).
 */
@Composable
fun DriverOnboardingRouter(viewModel: DriverOnboardingViewModel, strings: DriverStrings) {
    // DriverStrings IS-A RiderStrings, so the shared factories take the same resolver (the Android
    // idiom for iOS reusing RiderShared's L10n alongside DriverL10n).
    val rider = remember(strings) { RiderOnboardingScreens(strings) }
    val driver = remember(strings) { DriverOnboardingScreens(strings) }
    val scope = rememberCoroutineScope()
    var phone by remember { mutableStateOf("") }
    var code by remember { mutableStateOf("") }
    // In-step view acknowledgements (the core state is unchanged until their Continue is tapped).
    var recoveryCaptured by remember { mutableStateOf(false) }
    var permissionDeclinedAck by remember { mutableStateOf(false) }
    var autoUpdateEnabledAck by remember { mutableStateOf(false) }

    val adminName = viewModel.adminName
    when (viewModel.state) {
        OnboardingState.COMPLETE ->
            // Silent hand-off to the Driver's primary surface (home + Seat Toggle, a later spec).
            PrimarySurfacePlaceholder()

        OnboardingState.FRESH_INSTALL ->
            OnboardingScreenView(driver.driverIntro(onContinue = viewModel::begin))

        OnboardingState.PHONE_ENTRY ->
            // A Driver routed back here by an invalidated session sees the re-auth variant
            // (`auth.signin_again`); a fresh install sees the plain phone entry. Both are the core's
            // `PhoneEntry` state — only the leading copy differs (AC15 Driver branch).
            if (viewModel.reauthRequested) {
                OnboardingScreenView(
                    driver.reAuthPhoneEntry { scope.launch { viewModel.submitPhone(phone) } },
                    fieldValue = phone,
                    onFieldValueChange = { phone = it },
                )
            } else {
                OnboardingScreenView(
                    rider.phoneEntry { scope.launch { viewModel.submitPhone(phone) } },
                    fieldValue = phone,
                    onFieldValueChange = { phone = it },
                )
            }

        OnboardingState.PHONE_NOT_ON_FILE ->
            OnboardingScreenView(
                rider.phoneNotOnFile(adminName) { scope.launch { viewModel.submitPhone(phone) } },
                fieldValue = phone,
                onFieldValueChange = { phone = it },
            )

        OnboardingState.DEVICE_BINDING ->
            OnboardingScreenView(
                rider.deviceBinding(adminName) { scope.launch { viewModel.submitCode(code) } },
                fieldValue = code,
                onFieldValueChange = { code = it },
            )

        OnboardingState.BINDING_FAILED ->
            OnboardingScreenView(
                rider.bindingFailed(adminName) { scope.launch { viewModel.submitCode(code) } },
                fieldValue = code,
                onFieldValueChange = { code = it },
            )

        OnboardingState.PERMISSIONS -> {
            // Driver-only: capture the one-time Recovery Code first, right after the device is bound
            // and the session issued (ADR-0016 D3 / AC19), before the permissions ask. If no code is
            // available (degenerate shell case), skip it — never an empty-code screen, never a block.
            val recoveryCode = viewModel.recoveryCode
            if (!recoveryCaptured && recoveryCode != null) {
                OnboardingScreenView(driver.recoveryCodeCapture(recoveryCode) { recoveryCaptured = true })
            } else if (permissionDeclinedAck) {
                OnboardingScreenView(
                    rider.permissionsDeclined(adminName) {
                        scope.launch { viewModel.decideNotifications(allow = false) }
                    },
                )
            } else {
                OnboardingScreenView(
                    rider.permissions(
                        onAllow = { scope.launch { viewModel.decideNotifications(allow = true) } },
                        onDecline = { permissionDeclinedAck = true },
                    ),
                )
            }
        }

        OnboardingState.AUTO_UPDATE_STEP ->
            if (autoUpdateEnabledAck) {
                OnboardingScreenView(rider.autoUpdateEnabled(onContinue = viewModel::confirmAutoUpdate))
            } else {
                OnboardingScreenView(rider.autoUpdateStep(onContinue = { autoUpdateEnabledAck = true }))
            }

        // Calm degradation (O4 / AC8 / P10). NEEDS_REAUTH_HELP is unreachable for a Driver (the core
        // routes Drivers to PHONE_ENTRY) but is mapped for `when` exhaustiveness.
        OnboardingState.BELOW_MIN_VERSION, OnboardingState.NEEDS_REAUTH_HELP ->
            OnboardingScreenView(rider.calmHelp(adminName))
    }
}
