#!/usr/bin/env bash
#
# test-boundlessdriver.sh — run the Driver onboarding UI tests on the iOS simulator (spec 001 T12):
# snapshot ×4 a11y variants per screen + the Recovery-Code-capture / Driver-re-auth / no-signup /
# VoiceOver-order / AC14 logic tests. Builds the BoundlessKit XCFramework first (BoundlessDriver
# depends on it, directly and transitively via BoundlessRider).
#
# To (re)record snapshot baselines after an intentional UI change:
#   TEST_RUNNER_SNAPSHOT_RECORD=1 bash scripts/test-boundlessdriver.sh   # records, then re-run to verify
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKG="$REPO_ROOT/apple/BoundlessDriver"

if [[ -z "${DEVELOPER_DIR:-}" && -d /Applications/Xcode.app ]]; then
  export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi

# Pick a simulator: honor $BOUNDLESS_SIM, else the first available iPhone (robust across Xcode
# versions / CI runner images, where the exact model name varies).
SIM="${BOUNDLESS_SIM:-}"
if [[ -z "$SIM" ]]; then
  SIM="$(xcrun simctl list devices available \
    | sed -nE 's/^[[:space:]]+(iPhone[^()]*) \([0-9A-Fa-f-]+\) \(.*/\1/p' \
    | head -1 | sed 's/[[:space:]]*$//')"
  [[ -n "$SIM" ]] || {
    echo "error: no available iPhone simulator found. Install one in Xcode, or set BOUNDLESS_SIM." >&2
    exit 1
  }
fi
echo "▸ simulator: $SIM"

# BoundlessDriver depends on the BoundlessKit XCFramework (a build artifact) — build it first.
bash "$REPO_ROOT/scripts/build-boundlesskit.sh"

echo "▸ xcodebuild test (DriverShared) on '$SIM'"
cd "$PKG"
xcodebuild test \
  -scheme BoundlessDriver \
  -destination "platform=iOS Simulator,name=$SIM" \
  -derivedDataPath "$PKG/.build-xcode"
