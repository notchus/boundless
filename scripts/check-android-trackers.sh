#!/usr/bin/env bash
# Android dependency tracker gate (privacy invariant I8; spec 001 AC13).
#
# INTERIM gate: until a committed `gradle.lockfile` lets scripts/check-network-allowlist.sh scan
# the Android tree (deferred — see DEFERRED.md "Committed gradle.lockfile(s) …"), this resolves the
# Android modules' dependency graphs and greps the resolved closure against the same forbidden-
# tracker list, so a transitive bump that pulls in an analytics/crash SDK fails CI rather than
# going unnoticed. Requires the Android toolchain (runs in the `android` CI job); not locally gated.
set -euo pipefail
cd "$(dirname "$0")/.."

DENY=ci/forbidden-trackers.txt
DEPS="$(mktemp)"
trap 'rm -f "$DEPS"' EXIT

# Resolve every module's full dependency graph (compile + runtime + test) to one report.
( cd android && ./gradlew --no-daemon --console=plain \
    :core-bridge:dependencies :rider:app:dependencies :driver:app:dependencies ) >"$DEPS"

fail=0
patterns="$(grep -vE '^[[:space:]]*#|^[[:space:]]*$' "$DENY" || true)"
while IFS= read -r pat; do
    [ -n "$pat" ] || continue
    if grep -iFq -- "$pat" "$DEPS"; then
        echo "❌ forbidden tracker '$pat' referenced in the Android dependency tree (I8)" >&2
        fail=1
    fi
done <<<"$patterns"

if [ "$fail" -eq 1 ]; then
    echo "Android dependency tracker check failed (I8: no third-party analytics / trackers)." >&2
    exit 1
fi
echo "✓ Android dependency tree: no forbidden trackers (interim gate; gradle.lockfile scan is DEFERRED)."
