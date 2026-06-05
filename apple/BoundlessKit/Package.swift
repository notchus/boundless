// swift-tools-version: 6.0
//
// BoundlessKit — the Apple-platform binding to the Rust core (spec 001 T10-shell, Swift leg).
//
// `BoundlessKitFFI` is the UniFFI-generated XCFramework (the Rust core compiled for
// aarch64-apple-ios + -sim, plus its C header/modulemap). `BoundlessKit` is the generated
// Swift wrapper that consumers `import`. BOTH the xcframework (`Artifacts/`) and the wrapper
// (`Sources/BoundlessKit/boundless_ffi_swift.swift`) are GENERATED and git-ignored — run
// `scripts/build-boundlesskit.sh` first (it produces them from `core/ffi-swift`). They are
// reproducible from that Rust source and never hand-edited (P4 / ADR-0001, ADR-0022).
import PackageDescription

let package = Package(
    name: "BoundlessKit",
    platforms: [.iOS(.v17)],
    products: [
        .library(name: "BoundlessKit", targets: ["BoundlessKit"])
    ],
    targets: [
        // The Rust core as a binary xcframework (low-level C FFI; clang module
        // `boundless_ffi_swiftFFI`). Built by scripts/build-boundlesskit.sh.
        .binaryTarget(
            name: "BoundlessKitFFI",
            path: "Artifacts/BoundlessKitFFI.xcframework"
        ),
        // The generated Swift wrapper (`import BoundlessKit`).
        .target(
            name: "BoundlessKit",
            dependencies: ["BoundlessKitFFI"],
            path: "Sources/BoundlessKit"
        ),
        // Smoke test: proves the core state machine crosses Rust→UniFFI→Swift and runs on
        // the iOS simulator. (T11 adds swift-snapshot-testing for the Rider UI screens.)
        .testTarget(
            name: "BoundlessKitTests",
            dependencies: ["BoundlessKit"],
            path: "Tests/BoundlessKitTests"
        ),
    ]
)
