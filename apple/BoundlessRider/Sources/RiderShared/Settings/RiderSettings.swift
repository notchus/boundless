import SwiftUI

/// The Rider's Settings rows, as semantic identities. There is deliberately **no**
/// `automaticUpdates` case: O3 / AC6 require that automatic app updates are an OS-settings concern,
/// never surfaced in Boundless. The absence is therefore a *structural* guarantee — a row for it
/// cannot be constructed — not merely an omission a refactor could undo. Each row's visible title
/// comes from the catalog (P8).
public enum RiderSettingsRow: CaseIterable, Sendable {
    case notifications
    case help

    public var title: String {
        switch self {
        case .notifications: return L10n.settingsNotifications
        case .help: return L10n.settingsHelp
        }
    }

    /// VoiceOver/accessibility label for the row.
    public var a11yLabel: String { title }
}

/// The Rider Settings model. `surfacesAutomaticUpdatesToggle` is `false` by construction (AC6).
public struct RiderSettingsModel {
    public let rows: [RiderSettingsRow]

    public init(rows: [RiderSettingsRow] = RiderSettingsRow.allCases) {
        self.rows = rows
    }

    /// AC6: the Rider Settings surface never offers an automatic-updates toggle.
    public var surfacesAutomaticUpdatesToggle: Bool { false }

    public var a11yReadingOrder: [A11yDescriptor] {
        [A11yDescriptor(label: L10n.settingsTitle, traits: [.header])]
            + rows.map { A11yDescriptor(label: $0.a11yLabel, traits: [.button]) }
    }
}

/// Renders the Rider Settings list. Rows are navigation affordances (≥44pt). No toggles at all —
/// in particular, no automatic-updates toggle (AC6). The actual destinations (OS notification
/// settings, a "call your group" sheet) are the deferred app shell.
public struct RiderSettingsView: View {
    private let model: RiderSettingsModel

    public init(model: RiderSettingsModel = RiderSettingsModel()) {
        self.model = model
    }

    public var body: some View {
        List {
            ForEach(Array(model.rows.enumerated()), id: \.offset) { _, row in
                // A row is a plain navigational button — not a Toggle (AC6 holds by construction).
                Button(action: {}) {
                    HStack {
                        Text(verbatim: row.title)
                            .font(.body)
                            .foregroundStyle(.primary)
                        Spacer()
                        Image(systemName: "chevron.forward")
                            .foregroundStyle(.secondary)
                            .accessibilityHidden(true)
                    }
                    .frame(minHeight: 44)
                    .contentShape(Rectangle())
                }
                .accessibilityLabel(Text(verbatim: row.a11yLabel))
            }
        }
        .navigationTitle(Text(verbatim: L10n.settingsTitle))
    }
}
