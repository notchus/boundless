import Foundation

/// One element in a screen's VoiceOver reading order: its label and accessibility traits.
///
/// A screen derives its `a11yReadingOrder: [A11yDescriptor]` from the SAME `OnboardingScreenModel`
/// it renders (see `OnboardingScreenModel`), so the asserted reading order cannot drift from what
/// is drawn. Tests assert this list (labels present, headings marked, the auto-update confirmation
/// announced as a completed *state* not a button — AC11). swift-snapshot-testing has no
/// accessibility-tree strategy, so this model-level assertion IS the automatable reading-order
/// leg; the full recorded VoiceOver walkthrough remains a manual checklist item (plan §7).
public struct A11yDescriptor: Equatable, Sendable {
    public enum Trait: String, Equatable, Sendable, CaseIterable {
        case header
        case staticText
        case button
        case textField
    }

    public let label: String
    public let traits: Set<Trait>

    public init(label: String, traits: Set<Trait>) {
        self.label = label
        self.traits = traits
    }

    public var isHeader: Bool { traits.contains(.header) }
    public var isButton: Bool { traits.contains(.button) }
    public var isStaticText: Bool { traits.contains(.staticText) }
}
