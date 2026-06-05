# BoundlessKit — Apple binding to the Rust core

`BoundlessKit` is the [UniFFI](https://mozilla.github.io/uniffi-rs/)-generated Swift binding
to the Boundless Rust core (`core/ffi-swift`). The Apple clients (`apple/BoundlessRider`,
`apple/BoundlessDriver`, …) consume it so they **render** core decisions and never
re-implement the rules (constitution P4). Source of truth: `core/ffi-swift/src/lib.rs`.

## Generated, not committed

Two things in this package are **generated and git-ignored** — do not hand-edit, do not
expect them in a fresh checkout:

- `Artifacts/BoundlessKitFFI.xcframework` — the Rust core compiled for `aarch64-apple-ios`
  + `aarch64-apple-ios-sim`, with the C header + modulemap.
- `Sources/BoundlessKit/boundless_ffi_swift.swift` — the generated Swift wrapper.

Both are reproducible from `core/ffi-swift`. **Build them first:**

```sh
bash scripts/build-boundlesskit.sh      # cargo + uniffi-bindgen + xcodebuild -create-xcframework
```

## Test (iOS simulator)

```sh
bash scripts/test-boundlesskit.sh       # builds, then xcodebuild test on the iPhone 17 sim
```

The smoke test (`Tests/BoundlessKitTests`) proves the `core::auth` onboarding state machine
crosses Rust → UniFFI → Swift and runs on-device — on the first available iPhone simulator
(auto-detected; override with `BOUNDLESS_SIM="iPhone 17 Pro"`).

## Why mirror types (ADR-0022)

`core/ffi-swift` mirrors the core enums with `#[derive(uniffi::Enum)]` + exhaustive `From`
conversions rather than annotating the core directly, because the core crates must stay
`uniffi`-free to keep compiling to `wasm32-unknown-unknown`. The exhaustive `match` makes any
new core variant a compile error until mapped — a parity guarantee, not drift.

## Toolchain

Needs the Rust iOS targets (`rustup target add aarch64-apple-ios aarch64-apple-ios-sim`) and
Xcode. If Xcode is installed but not selected, the build script falls back to
`DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer` automatically. Versions are pinned
in `docs/stack-matrix.md` (uniffi 0.31.1).
