#!/usr/bin/env bash
#
# test-corebridge.sh — build the BoundlessCore binding, then run the Android harness checks
# (spec 001 Android bring-up). The Kotlin analog of test-boundlesskit.sh. Proves:
#   - Rust → UniFFI → Kotlin/JNA round-trips on the host JVM  (:core-bridge FFI smoke test)
#   - the Paparazzi snapshot harness verifies green             (:rider:app verifyPaparazziDebug)
#   - the apps build + link the per-ABI .so                     (:rider:app/:driver:app assembleDebug)
#
# Requires the Android toolchain (SDK + NDK + the 4 Rust Android targets + cargo-ndk). See the
# Android section of DEFERRED.md for the one-time bring-up; the `android` CI job installs it.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Build the generated Kotlin + per-ABI .so + host cdylib into :core-bridge.
bash "$REPO_ROOT/scripts/build-corebridge.sh"

echo "▸ Gradle: FFI smoke test + Paparazzi verify + assemble"
cd "$REPO_ROOT/android"
./gradlew --no-daemon --console=plain \
  :core-bridge:testDebugUnitTest \
  :rider:app:verifyPaparazziDebug \
  :rider:app:assembleDebug \
  :driver:app:assembleDebug
