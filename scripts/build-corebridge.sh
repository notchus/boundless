#!/usr/bin/env bash
#
# build-corebridge.sh — produce the BoundlessCore binding (generated Kotlin + per-ABI .so) from
# core/ffi-kotlin into the android/:core-bridge module (spec 001 Android bring-up). The Kotlin
# analog of build-boundlesskit.sh. Outputs are git-ignored and reproducible.
#
#   core/ffi-kotlin --uniffi-->  android/core-bridge/src/main/kotlin/uniffi/.../*.kt
#                   --cargo-ndk-> android/core-bridge/src/main/jniLibs/<abi>/libboundless_ffi_kotlin.so
#   (+ a HOST cdylib in core/target/release for the host-JVM smoke test's JNA load.)
#
# Requires: the 4 Rust Android targets + cargo-ndk + an installed NDK. See the Android section of
# DEFERRED.md for the one-time toolchain bring-up.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORE="$REPO_ROOT/core"
MODULE="$REPO_ROOT/android/core-bridge"
CRATE="boundless-ffi-kotlin"
LIB="boundless_ffi_kotlin" # cargo lib name (dashes → underscores)

# Host dynamic-lib extension (uniffi-bindgen library mode + the host-JVM test both load this).
case "$(uname -s)" in
  Darwin) HOST_EXT=dylib ;;
  *)      HOST_EXT=so ;;
esac

# Locate the Android SDK + NDK (env override wins; else the conventional macOS/Linux SDK path).
ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}"
export ANDROID_HOME
if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  # Prefer the pinned NDK; else the highest-versioned one installed (cargo-ndk also autodetects).
  PINNED_NDK="$ANDROID_HOME/ndk/28.2.13676358"
  if [[ -d "$PINNED_NDK" ]]; then
    ANDROID_NDK_HOME="$PINNED_NDK"
  elif [[ -d "$ANDROID_HOME/ndk" ]]; then
    ANDROID_NDK_HOME="$ANDROID_HOME/ndk/$(ls "$ANDROID_HOME/ndk" | sort -V | tail -1)"
  fi
  export ANDROID_NDK_HOME
fi
[[ -d "${ANDROID_NDK_HOME:-/nonexistent}" ]] || {
  echo "error: no Android NDK found. Set ANDROID_NDK_HOME or install one under \$ANDROID_HOME/ndk." >&2
  exit 1
}
command -v cargo-ndk >/dev/null || {
  echo "error: cargo-ndk not found. Run: cargo install cargo-ndk --version 4.1.2 --locked" >&2
  exit 1
}

KOTLIN_OUT="$MODULE/src/main/kotlin"
JNILIBS_OUT="$MODULE/src/main/jniLibs"

cd "$CORE"

echo "▸ [1/3] host cdylib (uniffi-bindgen library-mode input + host-JVM test JNA target)"
cargo build --release --features bindgen -p "$CRATE"

echo "▸ [2/3] generate Kotlin bindings → core-bridge/src/main/kotlin"
rm -rf "$KOTLIN_OUT"
mkdir -p "$KOTLIN_OUT"
cargo run --release --features bindgen --bin uniffi-bindgen -p "$CRATE" -- \
  generate --library "target/release/lib${LIB}.${HOST_EXT}" \
  --language kotlin --no-format --out-dir "$KOTLIN_OUT"

echo "▸ [3/3] cross-compile per-ABI .so → core-bridge/src/main/jniLibs (cargo-ndk)"
rm -rf "$JNILIBS_OUT"
mkdir -p "$JNILIBS_OUT"
cargo ndk -o "$JNILIBS_OUT" \
  -t arm64-v8a -t armeabi-v7a -t x86_64 -t x86 \
  build --release --lib -p "$CRATE"

echo "✓ BoundlessCore binding built:"
echo "    $KOTLIN_OUT/uniffi/${LIB}/${LIB}.kt"
echo "    $JNILIBS_OUT/{arm64-v8a,armeabi-v7a,x86_64,x86}/lib${LIB}.so"
echo "    $CORE/target/release/lib${LIB}.${HOST_EXT}  (host JVM smoke test)"
