import BoundlessKit
import SnapshotTesting
import SwiftUI
import XCTest

@testable import RiderShared

/// Shared fixtures for the Rider onboarding tests.
enum Fixtures {
    /// The admin's personal name, supplied by the manifest in production (ADR-0014).
    static let adminName = "Sarah"
}

extension XCTestCase {
    /// Records / verifies the **four required a11y snapshot variants** of a screen (a11y bar / AC11):
    /// default, largest Dynamic Type (`accessibility5`), dark mode, and RTL. Rendering is pinned to a
    /// fixed device config + tolerant `perceptualPrecision` for cross-machine stability. Recording is
    /// gated on the `SNAPSHOT_RECORD` env var, so baselines are produced once and committed; CI runs
    /// in verify mode (a missing baseline fails).
    @MainActor
    func assertA11ySnapshots(
        of view: some View,
        named name: String,
        layout: SwiftUISnapshotLayout = .device(config: .iPhone13),
        file: StaticString = #filePath,
        testName: String = #function,
        line: UInt = #line
    ) {
        let record: SnapshotTestingConfiguration.Record =
            ProcessInfo.processInfo.environment["SNAPSHOT_RECORD"] != nil ? .all : .never

        let variants: [(suffix: String, traits: UITraitCollection)] = [
            ("default", UITraitCollection()),
            ("largestText", UITraitCollection(preferredContentSizeCategory: .accessibilityExtraExtraExtraLarge)),
            ("dark", UITraitCollection(userInterfaceStyle: .dark)),
            ("rtl", UITraitCollection(layoutDirection: .rightToLeft)),
        ]

        withSnapshotTesting(record: record) {
            for variant in variants {
                assertSnapshot(
                    of: view,
                    as: .image(perceptualPrecision: 0.98, layout: layout, traits: variant.traits),
                    named: "\(name).\(variant.suffix)",
                    file: file,
                    testName: testName,
                    line: line
                )
            }
        }
    }
}

// MARK: - Injected-dependency stubs (the deferred app shell's boundaries)

@MainActor
final class StubNetworking: OnboardingNetworking {
    var signInResult: SignInResult
    var bindResult: BindResult

    init(signIn: SignInResult = .memberMatched, bind: BindResult = .bound) {
        self.signInResult = signIn
        self.bindResult = bind
    }

    func signIn(phone: String) async -> SignInResult { signInResult }
    func bindDevice(code: String) async -> BindResult { bindResult }
}

@MainActor
final class StubNotifications: NotificationPermissionRequesting {
    var granted: Bool
    init(granted: Bool) { self.granted = granted }
    func requestAuthorization() async -> Bool { granted }
}

@MainActor
final class StubManifest: ManifestProviding {
    var adminName: String?
    init(adminName: String?) { self.adminName = adminName }
}
