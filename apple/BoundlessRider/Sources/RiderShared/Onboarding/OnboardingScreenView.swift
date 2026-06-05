import SwiftUI

/// Renders an `OnboardingScreenModel`. One renderer for every onboarding screen, so layout,
/// Dynamic Type, contrast and dark mode are consistent and correct by construction (a11y bar).
/// Uses SwiftUI **semantic** colors and **built-in** button styles only — no hardcoded colors
/// (dark-mode-safe), no hardcoded point sizes for text (Dynamic Type via `.font` text styles).
/// The `ScrollView` lets every screen reflow at the largest Dynamic Type without clipping.
public struct OnboardingScreenView: View {
    private let model: OnboardingScreenModel

    public init(_ model: OnboardingScreenModel) {
        self.model = model
    }

    public var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: Spacing.large) {
                ForEach(Array(model.elements.enumerated()), id: \.offset) { _, element in
                    elementView(element)
                }

                if let field = model.field {
                    fieldView(field)
                }

                ForEach(Array(model.actions.enumerated()), id: \.offset) { _, action in
                    actionButton(action)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(Spacing.large)
        }
        .background(Color(.systemBackground))
    }

    @ViewBuilder
    private func elementView(_ element: BodyElement) -> some View {
        switch element {
        case let .heading(text):
            Text(verbatim: text)
                .font(.largeTitle)
                .fontWeight(.semibold)
                .foregroundStyle(.primary)
                .accessibilityAddTraits(.isHeader)
                .fixedSize(horizontal: false, vertical: true)
        case let .paragraph(text):
            Text(verbatim: text)
                .font(.body)
                .foregroundStyle(.primary)
                .fixedSize(horizontal: false, vertical: true)
        case let .banner(text):
            // Icon + text — never color-only (a11y bar). Calm, not alarming.
            Label {
                Text(verbatim: text)
                    .font(.body)
                    .foregroundStyle(.primary)
                    .fixedSize(horizontal: false, vertical: true)
            } icon: {
                Image(systemName: "info.circle")
            }
            .labelStyle(.titleAndIcon)
            .padding(Spacing.medium)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color(.secondarySystemBackground), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        case let .confirmation(text):
            // A completed state — static text + checkmark, NOT a button (a11y notes; AC5).
            Label {
                Text(verbatim: text)
                    .font(.title3)
                    .foregroundStyle(.primary)
                    .fixedSize(horizontal: false, vertical: true)
            } icon: {
                Image(systemName: "checkmark.circle")
            }
            .labelStyle(.titleAndIcon)
            .accessibilityElement(children: .combine)
        }
    }

    private func fieldView(_ field: FieldModel) -> some View {
        // Empty visible placeholder — the heading above is the visible label; VoiceOver uses the
        // accessibilityLabel. (Empty "" is not user-visible copy, so no catalog key is needed.)
        TextField("", text: field.text)
            .textFieldStyle(.roundedBorder)
            .font(.title3)
            .keyboardType(field.kind == .phone ? .phonePad : .numberPad)
            .textContentType(field.kind == .phone ? .telephoneNumber : .oneTimeCode)
            .accessibilityLabel(Text(verbatim: field.label))
            .frame(minHeight: 44)
            .disabled(model.isOffline)
    }

    private func actionButton(_ action: ScreenAction) -> some View {
        Button(action: action.perform) {
            Text(verbatim: action.label)
                .font(.headline)
                .frame(maxWidth: .infinity, minHeight: 44)
        }
        .modifier(EmphasisStyle(emphasis: action.emphasis))
        .controlSize(.large)
        .disabled(model.isOffline)
    }
}

/// Applies the built-in prominent / bordered button style by emphasis (keeps the call site clean
/// and avoids hardcoded colors — the styles derive from the accent tint + system contrast).
private struct EmphasisStyle: ViewModifier {
    let emphasis: ScreenAction.Emphasis
    func body(content: Content) -> some View {
        switch emphasis {
        case .primary: content.buttonStyle(.borderedProminent)
        case .secondary: content.buttonStyle(.bordered)
        }
    }
}
