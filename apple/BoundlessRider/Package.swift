// swift-tools-version: 6.0
//
// BoundlessRider — the Rider's native iOS onboarding UI (spec 001 T11).
//
// `RiderShared` is the testable feature module: every onboarding screen rendered from the
// `core::auth` state machine exported by `BoundlessKit` (the Swift RENDERS core decisions and
// never re-implements the rules — constitution P4 / ADR-0001). Driven on the iOS simulator via
// `xcodebuild test`; there is no `.xcodeproj` app bundle here — the shippable app shell, the
// OpenAPI HTTP client, Keychain storage, APNs and the signed-manifest fetch are the imperative
// shell (DEFERRED.md → "Apple / T11-shell"), deferred behind injected protocols.
//
// BoundlessKit is a local path dependency; its XCFramework is a build artifact produced by
// `scripts/build-boundlesskit.sh` (run `scripts/test-boundlessrider.sh`, which builds it first).
import PackageDescription

let package = Package(
    name: "BoundlessRider",
    // Required because RiderShared ships a localized resource (the String Catalog, P8).
    defaultLocalization: "en",
    platforms: [.iOS(.v17)],
    products: [
        .library(name: "RiderShared", targets: ["RiderShared"])
    ],
    dependencies: [
        .package(path: "../BoundlessKit"),
        // Snapshot tests for the four required a11y variants (AC11). Exact-pinned: the lock
        // (Package.resolved) is ground truth; docs/stack-matrix.md mirrors it. MIT.
        .package(
            url: "https://github.com/pointfreeco/swift-snapshot-testing",
            exact: "1.19.2"
        ),
    ],
    targets: [
        .target(
            name: "RiderShared",
            dependencies: [
                .product(name: "BoundlessKit", package: "BoundlessKit")
            ],
            path: "Sources/RiderShared",
            resources: [
                // The String Catalog is not auto-detected by SwiftPM — declare it explicitly.
                .process("Localization/Onboarding.xcstrings")
            ]
        ),
        .testTarget(
            name: "RiderSharedTests",
            dependencies: [
                "RiderShared",
                .product(name: "SnapshotTesting", package: "swift-snapshot-testing"),
            ],
            path: "Tests/RiderSharedTests"
        ),
    ]
)
