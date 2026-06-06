package app.boundless.rider.onboarding

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import app.boundless.rider.i18n.RiderStrings
import kotlinx.coroutines.launch
import uniffi.boundless_ffi_kotlin.OnboardingState

/**
 * Maps the current [OnboardingState] to the screen the Rider sees, wiring each affordance to the
 * view model. The router owns only *view* state (the field text and the two in-step
 * acknowledgements); the authoritative [OnboardingState] lives in the view model and is advanced
 * solely by the core (P4). Completion is **silent** — `COMPLETE` routes to the primary surface with
 * no "all set" screen (voice-and-tone). The Android twin of `RiderShared.OnboardingRouter`.
 *
 * The composition root (MainActivity, deferred to T13-shell) supplies the [strings] resolver
 * (`AndroidRiderStrings`) and constructs the [viewModel] with the real conformers.
 */
@Composable
fun OnboardingRouter(viewModel: OnboardingViewModel, strings: RiderStrings) {
    val screens = remember(strings) { RiderOnboardingScreens(strings) }
    val scope = rememberCoroutineScope()
    var phone by remember { mutableStateOf("") }
    var code by remember { mutableStateOf("") }
    // In-step view acknowledgements (the core state is unchanged until their Continue is tapped).
    var permissionDeclinedAck by remember { mutableStateOf(false) }
    var autoUpdateEnabledAck by remember { mutableStateOf(false) }

    val adminName = viewModel.adminName
    when (viewModel.state) {
        OnboardingState.COMPLETE ->
            // Silent completion — the hand-off placeholder, no "all set" celebration.
            PrimarySurfacePlaceholder()

        OnboardingState.FRESH_INSTALL ->
            OnboardingScreenView(screens.helperIntro(onContinue = viewModel::begin))

        OnboardingState.PHONE_ENTRY ->
            OnboardingScreenView(
                screens.phoneEntry { scope.launch { viewModel.submitPhone(phone) } },
                fieldValue = phone,
                onFieldValueChange = { phone = it },
            )

        OnboardingState.PHONE_NOT_ON_FILE ->
            OnboardingScreenView(
                screens.phoneNotOnFile(adminName) { scope.launch { viewModel.submitPhone(phone) } },
                fieldValue = phone,
                onFieldValueChange = { phone = it },
            )

        OnboardingState.DEVICE_BINDING ->
            OnboardingScreenView(
                screens.deviceBinding(adminName) { scope.launch { viewModel.submitCode(code) } },
                fieldValue = code,
                onFieldValueChange = { code = it },
            )

        OnboardingState.BINDING_FAILED ->
            OnboardingScreenView(
                screens.bindingFailed(adminName) { scope.launch { viewModel.submitCode(code) } },
                fieldValue = code,
                onFieldValueChange = { code = it },
            )

        OnboardingState.PERMISSIONS ->
            if (permissionDeclinedAck) {
                OnboardingScreenView(
                    screens.permissionsDeclined(adminName) {
                        scope.launch { viewModel.decideNotifications(allow = false) }
                    },
                )
            } else {
                OnboardingScreenView(
                    screens.permissions(
                        onAllow = { scope.launch { viewModel.decideNotifications(allow = true) } },
                        onDecline = { permissionDeclinedAck = true },
                    ),
                )
            }

        OnboardingState.AUTO_UPDATE_STEP ->
            if (autoUpdateEnabledAck) {
                OnboardingScreenView(screens.autoUpdateEnabled(onContinue = viewModel::confirmAutoUpdate))
            } else {
                OnboardingScreenView(screens.autoUpdateStep(onContinue = { autoUpdateEnabledAck = true }))
            }

        // Terminal calm screens — no CTA, no form (AC8 / AC15 / P10).
        OnboardingState.BELOW_MIN_VERSION, OnboardingState.NEEDS_REAUTH_HELP ->
            OnboardingScreenView(screens.calmHelp(adminName))
    }
}
