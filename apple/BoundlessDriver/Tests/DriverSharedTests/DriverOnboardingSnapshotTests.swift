import RiderShared
import SnapshotTesting
import SwiftUI
import XCTest

@testable import DriverShared

/// The a11y snapshot matrix (AC11): **every Driver onboarding screen** × {default, largest text,
/// dark, RTL}. The Driver reuses `RiderShared`'s renderer for the role-neutral steps, so those
/// baselines render identically to the Rider's — but they are the **Driver app's own** baselines,
/// independently closing AC11 for this platform (if the shared renderer ever changes, both apps'
/// baselines update together, which is correct). Also closes the snapshot legs of AC5 (auto-update
/// enabled), AC8 (calm below-min, no CTA), AC14 (declined permission) and AC19 (Recovery Code
/// capture). Strings come from the catalogs (P8); the screens render the core state machine (P4).
@MainActor
final class DriverOnboardingSnapshotTests: XCTestCase {
    /// Sweeps the full Driver screen set through the four required variants. Each screen's baselines
    /// are uniquely named, so they don't collide despite sharing one test function.
    func testEveryDriverScreenMatrix() {
        for (name, model) in DriverScreenFixtures.allModels() {
            assertA11ySnapshots(of: OnboardingScreenView(model), named: name)
        }
    }

    /// Silent completion: the shared hand-off placeholder, with no "all set" celebration
    /// (voice-and-tone). The Driver lands on its home (Seat Toggle) — a later spec.
    func testPrimarySurfacePlaceholder() {
        assertA11ySnapshots(of: PrimarySurfacePlaceholderView(), named: "primarySurface")
    }
}
