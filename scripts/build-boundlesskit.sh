#!/usr/bin/env bash
#
# build-boundlesskit.sh — produce the BoundlessKit XCFramework + Swift wrapper from
# core/ffi-swift (spec 001 T10-shell, Swift leg). Outputs are git-ignored and reproducible.
#
#   core/ffi-swift  --uniffi-->  apple/BoundlessKit/{Artifacts/*.xcframework,
#                                                    Sources/BoundlessKit/boundless_ffi_swift.swift}
#
# Requires: Rust iOS targets (aarch64-apple-ios{,-sim}) + Xcode. If Xcode is installed but not
# selected (and sudo is unavailable), we point DEVELOPER_DIR at it. Pins: uniffi 0.31.1.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORE="$REPO_ROOT/core"
PKG="$REPO_ROOT/apple/BoundlessKit"
CRATE="boundless-ffi-swift"
LIB="boundless_ffi_swift" # cargo lib name (dashes → underscores)

# Use the installed Xcode toolchain (this dev env: Xcode present but not xcode-select'd).
if [[ -z "${DEVELOPER_DIR:-}" && -d /Applications/Xcode.app ]]; then
  export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi
command -v xcodebuild >/dev/null || {
  echo "error: xcodebuild not found (need Xcode, not just Command Line Tools)" >&2
  exit 1
}

# Preflight: the iOS Rust targets must be installed (CI adds them; a local run may not have them).
for t in aarch64-apple-ios aarch64-apple-ios-sim; do
  rustup target list --installed 2>/dev/null | grep -qx "$t" || {
    echo "error: missing Rust target '$t'. Run:" >&2
    echo "    rustup target add aarch64-apple-ios aarch64-apple-ios-sim" >&2
    exit 1
  }
done

SRC_DIR="$PKG/Sources/BoundlessKit"
ARTIFACTS="$PKG/Artifacts"
XCF="$ARTIFACTS/BoundlessKitFFI.xcframework"
HDRS="$(mktemp -d)"
trap 'rm -rf "$HDRS"' EXIT

cd "$CORE"

echo "▸ [1/5] host cdylib (uniffi-bindgen library-mode input)"
cargo build --release --features bindgen -p "$CRATE"

echo "▸ [2/5] generate Swift bindings"
rm -rf "$SRC_DIR"
mkdir -p "$SRC_DIR"
cargo run --release --features bindgen --bin uniffi-bindgen -p "$CRATE" -- \
  generate --library "target/release/lib${LIB}.dylib" \
  --language swift --no-format --out-dir "$SRC_DIR"

echo "▸ [3/5] stage headers (rename *FFI.modulemap → module.modulemap for Xcode)"
# Guard the expected uniffi output names: a uniffi bump can change them, and a bare `mv`
# failure under `set -e` is opaque. Point the next person at the likely cause.
for f in "${LIB}FFI.modulemap" "${LIB}FFI.h"; do
  [[ -f "$SRC_DIR/$f" ]] || {
    echo "error: expected uniffi output '$f' not found in $SRC_DIR." >&2
    echo "       The uniffi output naming may have changed (pinned uniffi 0.31.1)." >&2
    exit 1
  }
done
mv "$SRC_DIR/${LIB}FFI.modulemap" "$HDRS/module.modulemap"
mv "$SRC_DIR/${LIB}FFI.h" "$HDRS/${LIB}FFI.h"
# Only the Swift wrapper remains in the package source dir.

echo "▸ [4/5] build iOS device + simulator staticlibs (--lib: no bindgen/clap in the .a)"
cargo build --release --lib --target aarch64-apple-ios -p "$CRATE"
cargo build --release --lib --target aarch64-apple-ios-sim -p "$CRATE"

echo "▸ [5/5] assemble xcframework"
rm -rf "$XCF"
mkdir -p "$ARTIFACTS"
xcodebuild -create-xcframework \
  -library "target/aarch64-apple-ios/release/lib${LIB}.a" -headers "$HDRS" \
  -library "target/aarch64-apple-ios-sim/release/lib${LIB}.a" -headers "$HDRS" \
  -output "$XCF" >/dev/null

echo "✓ BoundlessKit built:"
echo "    $XCF"
echo "    $SRC_DIR/${LIB}.swift"
