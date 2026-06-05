#!/usr/bin/env bash
#
# test-boundlesskit.sh — build BoundlessKit, then run its smoke test on the iOS simulator
# (spec 001 T10-shell). Proves the Rust core crosses Rust → UniFFI → Swift and runs on-device.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKG="$REPO_ROOT/apple/BoundlessKit"

if [[ -z "${DEVELOPER_DIR:-}" && -d /Applications/Xcode.app ]]; then
  export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi

# Pick a simulator: honor $BOUNDLESS_SIM, else the first available iPhone (robust across
# Xcode versions / CI runner images, where the exact model name varies).
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

bash "$REPO_ROOT/scripts/build-boundlesskit.sh"

echo "▸ xcodebuild test on '$SIM'"
cd "$PKG"
xcodebuild test \
  -scheme BoundlessKit \
  -destination "platform=iOS Simulator,name=$SIM" \
  -derivedDataPath "$PKG/.build-xcode"
