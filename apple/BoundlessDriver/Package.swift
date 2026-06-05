// swift-tools-version: 6.0
//
// BoundlessDriver — the Driver's native iOS onboarding UI (spec 001 T12).
//
// `DriverShared` is the testable feature module: the Driver self-onboarding flow rendered from the
// `core::auth` state machine exported by `BoundlessKit` (the Swift RENDERS core decisions and never
// re-implements the rules — constitution P4 / ADR-0001). It reuses the shared onboarding kit from
// `RiderShared` (the screen model + single renderer + L10n + the injected protocols + the role-neutral
// screen factories) and adds only the Driver deltas: a self-onboard intro, the one-time **Recovery
// Code capture** screen (ADR-0016 D3 / AC19), and the interactive re-auth `PhoneEntry`
// (`auth.signin_again`) that a Driver — unlike a Rider — reaches when a session expires.
//
// Driven on the iOS simulator via `xcodebuild test`; there is no `.xcodeproj` app bundle here — the
// shippable app shell, the OpenAPI HTTP client (incl. /api/auth/recovery/rebind), the real
// Recovery-Code provider, Keychain storage, APNs and the signed-manifest fetch are the imperative
// shell (DEFERRED.md → "Apple / Driver UI — T12-shell"), deferred behind injected protocols.
//
// BoundlessKit is a local path dependency; its XCFramework is a build artifact produced by
// `scripts/build-boundlesskit.sh` (run `scripts/test-boundlessdriver.sh`, which builds it first).
import PackageDescription

let package = Package(
    name: "BoundlessDriver",
    // Required because DriverShared ships a localized resource (the String Catalog, P8).
    defaultLocalization: "en",
    platforms: [.iOS(.v17)],
    products: [
        .library(name: "DriverShared", targets: ["DriverShared"])
    ],
    dependencies: [
        .package(path: "../BoundlessKit"),
        // The shared onboarding kit (screen model/renderer, L10n, OnboardingViewModel, protocols,
        // role-neutral screen factories). Reused, not duplicated (P4).
        .package(path: "../BoundlessRider"),
        // Snapshot tests for the four required a11y variants (AC11). Exact-pinned: the lock
        // (Package.resolved) is ground truth; docs/stack-matrix.md mirrors it. MIT.
        .package(
            url: "https://github.com/pointfreeco/swift-snapshot-testing",
            exact: "1.19.2"
        ),
    ],
    targets: [
        .target(
            name: "DriverShared",
            dependencies: [
                .product(name: "BoundlessKit", package: "BoundlessKit"),
                .product(name: "RiderShared", package: "BoundlessRider"),
            ],
            path: "Sources/DriverShared",
            resources: [
                // The String Catalog is not auto-detected by SwiftPM — declare it explicitly.
                .process("Localization/DriverOnboarding.xcstrings")
            ]
        ),
        .testTarget(
            name: "DriverSharedTests",
            dependencies: [
                "DriverShared",
                .product(name: "SnapshotTesting", package: "swift-snapshot-testing"),
            ],
            path: "Tests/DriverSharedTests"
        ),
    ]
)
