import BoundlessKit
import RiderShared
import SwiftUI

/// Maps the current `OnboardingState` to the screen the Driver sees, wiring each affordance to the
/// view model. Like the Rider router, it owns only *view* state (the field text and the in-step
/// acknowledgements); the authoritative `OnboardingState` lives in the view model and is advanced
/// solely by the core (P4). The role-neutral steps reuse `RiderOnboardingScreens` verbatim; the
/// Driver-only screens come from `DriverOnboardingScreens`. Completion is **silent** — `.complete`
/// routes to the primary surface with no "all set" screen (voice-and-tone).
public struct DriverOnboardingRouter: View {
    @State private var viewModel: DriverOnboardingViewModel
    @State private var phone = ""
    @State private var code = ""
    // In-step view acknowledgements (the core state is unchanged until their Continue is tapped).
    @State private var recoveryCaptured = false
    @State private var permissionDeclinedAck = false
    @State private var autoUpdateEnabledAck = false

    public init(viewModel: DriverOnboardingViewModel) {
        _viewModel = State(initialValue: viewModel)
    }

    public var body: some View {
        Group {
            if viewModel.state == .complete {
                // Silent hand-off to the Driver's primary surface (home + Seat Toggle, a later spec).
                // Reuses the shared neutral placeholder — "no celebration of plumbing".
                PrimarySurfacePlaceholderView()
            } else {
                OnboardingScreenView(currentModel)
            }
        }
    }

    private var adminName: String? { viewModel.adminName }

    private var currentModel: OnboardingScreenModel {
        switch viewModel.state {
        case .freshInstall:
            return DriverOnboardingScreens.driverIntro(onContinue: viewModel.begin)

        case .phoneEntry:
            // A Driver routed back here by an invalidated session sees the re-auth variant
            // (`auth.signin_again`); a fresh install sees the plain phone entry. Both are the
            // core's `PhoneEntry` state — only the leading copy differs (AC15 Driver branch).
            if viewModel.reauthRequested {
                return DriverOnboardingScreens.reAuthPhoneEntry(text: $phone) {
                    Task { await viewModel.submitPhone(phone) }
                }
            }
            return RiderOnboardingScreens.phoneEntry(text: $phone) {
                Task { await viewModel.submitPhone(phone) }
            }

        case .phoneNotOnFile:
            return RiderOnboardingScreens.phoneNotOnFile(text: $phone, adminName: adminName) {
                Task { await viewModel.submitPhone(phone) }
            }

        case .deviceBinding:
            return RiderOnboardingScreens.deviceBinding(text: $code, adminName: adminName) {
                Task { await viewModel.submitCode(code) }
            }

        case .bindingFailed:
            return RiderOnboardingScreens.bindingFailed(text: $code, adminName: adminName) {
                Task { await viewModel.submitCode(code) }
            }

        case .permissions:
            // Driver-only: capture the one-time Recovery Code first, right after the device is bound
            // and the session issued (spec §C; ADR-0016 D3 / AC19), before the permissions ask. If no
            // code is available (degenerate shell case), skip it — never an empty-code screen, never
            // a block.
            if !recoveryCaptured, let recoveryCode = viewModel.recoveryCode {
                return DriverOnboardingScreens.recoveryCodeCapture(code: recoveryCode) {
                    recoveryCaptured = true
                }
            }
            if permissionDeclinedAck {
                return RiderOnboardingScreens.permissionsDeclined(adminName: adminName) {
                    Task { await viewModel.decideNotifications(allow: false) }
                }
            }
            return RiderOnboardingScreens.permissions(
                onAllow: { Task { await viewModel.decideNotifications(allow: true) } },
                onDecline: { permissionDeclinedAck = true }
            )

        case .autoUpdateStep:
            if autoUpdateEnabledAck {
                return RiderOnboardingScreens.autoUpdateEnabled(onContinue: viewModel.confirmAutoUpdate)
            }
            return RiderOnboardingScreens.autoUpdateStep(onContinue: { autoUpdateEnabledAck = true })

        case .belowMinVersion:
            // Calm degradation — no CTA, no form (O4 / AC8 / P10). A Driver can reach this from any
            // auth response just like a Rider.
            return RiderOnboardingScreens.calmHelp(adminName: adminName)

        case .needsReauthHelp:
            // Unreachable for a Driver (re-auth routes to `PhoneEntry`); mapped for exhaustiveness.
            return RiderOnboardingScreens.calmHelp(adminName: adminName)

        case .complete:
            // Unreachable: `.complete` is handled above by the placeholder. Kept for exhaustiveness.
            return RiderOnboardingScreens.calmHelp(adminName: adminName)
        }
    }
}
