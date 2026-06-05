import SwiftUI

/// A body element of an onboarding screen. Pure data, so the SAME value both renders (in
/// `OnboardingScreenView`) and produces the screen's a11y reading order — the two cannot drift.
public enum BodyElement: Equatable, Sendable {
    /// The screen title — rendered large, marked as a VoiceOver header.
    case heading(String)
    /// Explanatory copy.
    case paragraph(String)
    /// A calm info / recovery banner (icon + text — never color-only; a11y bar).
    case banner(String)
    /// A completed-state announcement (e.g. "Automatic updates are on.") — static text with a
    /// checkmark, announced as a state, NOT a button (a11y notes; AC5).
    case confirmation(String)
}

/// A tappable affordance — the single large control(s) per step (a11y notes). Holds a closure,
/// so it is intentionally not `Equatable`.
public struct ScreenAction {
    public enum Emphasis { case primary, secondary }
    public let label: String
    public let emphasis: Emphasis
    public let perform: () -> Void

    public init(label: String, emphasis: Emphasis = .primary, perform: @escaping () -> Void) {
        self.label = label
        self.emphasis = emphasis
        self.perform = perform
    }
}

/// An optional single text entry (phone number / Onboarding Code). The binding is owned by the
/// router (live) or `.constant("")` (snapshots). `isEnabled == false` (via the model's `isOffline`)
/// models the Offline overlay: the sign-in UI is shown but the network-dependent action is deferred
/// until connectivity — the overlay reuses the sign-in copy, so it adds no new catalog keys.
public struct FieldModel {
    public enum Entry { case phone, code }
    public let label: String
    public let kind: Entry
    public let text: Binding<String>

    public init(label: String, kind: Entry, text: Binding<String>) {
        self.label = label
        self.kind = kind
        self.text = text
    }
}

/// The complete description of one onboarding screen: what to render AND (derived) its a11y order.
public struct OnboardingScreenModel {
    public let elements: [BodyElement]
    public let field: FieldModel?
    public let actions: [ScreenAction]
    /// Offline overlay over a sign-in/binding step: the action is shown disabled (deferred until
    /// connectivity). Only set where `BoundlessKit.allowsOfflineOverlay(state:)` is true.
    public let isOffline: Bool

    public init(
        elements: [BodyElement],
        field: FieldModel? = nil,
        actions: [ScreenAction] = [],
        isOffline: Bool = false
    ) {
        self.elements = elements
        self.field = field
        self.actions = actions
        self.isOffline = isOffline
    }

    /// The VoiceOver reading order, derived from the rendered content (no drift). Tests assert it.
    public var a11yReadingOrder: [A11yDescriptor] {
        var order: [A11yDescriptor] = elements.map { element in
            switch element {
            case let .heading(text): return A11yDescriptor(label: text, traits: [.header])
            case let .paragraph(text): return A11yDescriptor(label: text, traits: [.staticText])
            case let .banner(text): return A11yDescriptor(label: text, traits: [.staticText])
            case let .confirmation(text): return A11yDescriptor(label: text, traits: [.staticText])
            }
        }
        if let field {
            order.append(A11yDescriptor(label: field.label, traits: [.textField]))
        }
        order += actions.map { A11yDescriptor(label: $0.label, traits: [.button]) }
        return order
    }

    /// Whether this screen exposes any text-entry / sign-in affordance. AC15 (NeedsReauthHelp shows
    /// no form) and AC1(b) (no signup route) assert this is `false` for the calm/terminal screens.
    public var hasInputAffordance: Bool { field != nil }

    /// All affordance labels on the screen — used by AC8 ("no Update Now") and AC1(b) inspections.
    public var actionLabels: [String] { actions.map(\.label) }
}
