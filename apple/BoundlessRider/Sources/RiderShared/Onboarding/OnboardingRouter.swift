import BoundlessKit
import SwiftUI

/// Maps the current `OnboardingState` to the screen the Rider sees, wiring each affordance to the
/// view model. The router owns only *view* state (the field text and the two in-step
/// acknowledgements); the authoritative `OnboardingState` lives in the view model and is advanced
/// solely by the core (P4). Completion is **silent** — `.complete` routes to the primary surface
/// with no "all set" screen (voice-and-tone).
public struct OnboardingRouter: View {
    @State private var viewModel: OnboardingViewModel
    @State private var phone = ""
    @State private var code = ""
    // In-step view acknowledgements (the core state is unchanged until their Continue is tapped).
    @State private var permissionDeclinedAck = false
    @State private var autoUpdateEnabledAck = false

    public init(viewModel: OnboardingViewModel) {
        _viewModel = State(initialValue: viewModel)
    }

    public var body: some View {
        Group {
            if viewModel.state == .complete {
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
            return RiderOnboardingScreens.helperIntro(onContinue: viewModel.begin)

        case .phoneEntry:
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

        case .belowMinVersion, .needsReauthHelp:
            // Terminal calm screen — no CTA, no form (AC8 / AC15 / P10).
            return RiderOnboardingScreens.calmHelp(adminName: adminName)

        case .complete:
            // Unreachable: `.complete` is handled above by the placeholder. Kept for exhaustiveness.
            return RiderOnboardingScreens.calmHelp(adminName: adminName)
        }
    }
}

/// The hand-off after a **silent** onboarding completion. The real Rider primary surface ("You're
/// coming tonight") is a later spec; this neutral placeholder asserts the constitution's "no
/// celebration of plumbing" — there is deliberately no "all set" copy here.
public struct PrimarySurfacePlaceholderView: View {
    public init() {}
    public var body: some View {
        Color(.systemBackground)
            .ignoresSafeArea()
            .accessibilityHidden(true)
    }
}
