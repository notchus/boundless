#!/usr/bin/env bash
# Meta-test for the binding-drift gate (spec 001 task T01 acceptance).
# Proves scripts/check-binding-drift.sh PASSES on a clean tree and FAILS on a deliberate
# ungenerated change — both an input change (core/api) and a hand-edited generated file.
# Leaves the working tree pristine (restores via a snapshot on exit).
set -euo pipefail
cd "$(dirname "$0")/.."

CHECK="scripts/check-binding-drift.sh"
IN=api/openapi.yaml
# Perturb a REAL generated file, not a `.gitkeep` placeholder — otherwise "hand-edit a
# generated file" would be a vacuous test. Fail loudly if the tree has only placeholders.
GEN="$(find api/generated -type f ! -name '.gitkeep' | head -1)"
if [ -z "$GEN" ]; then
    echo "❌ META-TEST FAILED: no non-placeholder file under api/generated/ to perturb." >&2
    exit 1
fi
LOCK=api/.bindings.lock

TMP="$(mktemp -d)"
cp "$IN" "$TMP/in"
cp "$GEN" "$TMP/gen"
cp "$LOCK" "$TMP/lock"
restore() { cp "$TMP/in" "$IN"; cp "$TMP/gen" "$GEN"; cp "$TMP/lock" "$LOCK"; rm -rf "$TMP"; }
trap restore EXIT

fail() { echo "❌ META-TEST FAILED: $1" >&2; exit 1; }

# 0. clean tree must pass
bash "$CHECK" >/dev/null 2>&1 || fail "clean tree should PASS the drift check"

# 1. input drift (contract/core changed, bindings not regenerated) must fail
printf '\n# meta-test drift marker (transient)\n' >>"$IN"
if bash "$CHECK" >/dev/null 2>&1; then fail "input change without regen should FAIL"; fi
cp "$TMP/in" "$IN"

# 2. hand-edited generated file must fail
printf '\nmeta-test hand-edit (transient)\n' >>"$GEN"
if bash "$CHECK" >/dev/null 2>&1; then fail "hand-edited generated file should FAIL"; fi
cp "$TMP/gen" "$GEN"

# 3. restored tree must pass again
bash "$CHECK" >/dev/null 2>&1 || fail "restored tree should PASS"

echo "✓ binding-drift meta-test passed (clean→pass · input-drift→fail · generated-handedit→fail)."
