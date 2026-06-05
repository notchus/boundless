#!/usr/bin/env bash
# Boundless network allow-list / no-tracker gate (privacy invariant I8; spec 001 AC13).
# Scans dependency lock files for forbidden third-party analytics / tracker / crash SDKs.
# Greenfield-tolerant: passes (vacuously) when no lock file exists yet.
set -euo pipefail
cd "$(dirname "$0")/.."

DENY=ci/forbidden-trackers.txt
fail=0

locks="$(
    {
        find . -name 'Cargo.lock' -not -path '*/target/*'
        find . -name 'pnpm-lock.yaml' -not -path '*/node_modules/*'
        find . -name 'package-lock.json' -not -path '*/node_modules/*'
        find . -name 'Package.resolved' -not -path '*/.build/*' -not -path '*/.build-xcode/*' -not -path '*/DerivedData/*'
        find . -name 'gradle.lockfile'
    } 2>/dev/null | LC_ALL=C sort -u
)"

if [ -z "$locks" ]; then
    echo "ℹ no dependency lock files yet — network allow-list is vacuously green (greenfield)."
    exit 0
fi

patterns="$(grep -vE '^[[:space:]]*#|^[[:space:]]*$' "$DENY" || true)"

while IFS= read -r lock; do
    [ -n "$lock" ] || continue
    while IFS= read -r pat; do
        [ -n "$pat" ] || continue
        if grep -iFq -- "$pat" "$lock"; then
            echo "❌ forbidden tracker '$pat' referenced in $lock (I8)" >&2
            fail=1
        fi
    done <<<"$patterns"
done <<<"$locks"

if [ "$fail" -eq 1 ]; then
    echo "Network allow-list check failed (I8: no third-party analytics / trackers)." >&2
    exit 1
fi

n="$(printf '%s\n' "$locks" | grep -c .)"
echo "✓ network allow-list: scanned ${n} lock file(s); no forbidden trackers."
exit 0
