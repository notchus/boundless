#!/usr/bin/env bash
# Boundless binding-drift gate (ADR-0001). Fails if EITHER:
#   (a) any file under api/generated/** was hand-edited — generated files are never edited
#       by hand; or
#   (b) the contracts/core changed (api/openapi.yaml, api/boundless.proto, core/**)
#       without regenerated bindings.
# Remediation for both: `bash scripts/generate-bindings.sh` then commit the result.
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck source=scripts/_bindings_common.sh
. scripts/_bindings_common.sh

LOCK=api/.bindings.lock
if [ ! -f "$LOCK" ]; then
    echo "❌ $LOCK missing. Run: bash scripts/generate-bindings.sh" >&2
    exit 1
fi

# Extract a recorded section ([inputs] or [outputs]) from the lock.
lock_block() {
    awk -v want="[$1]" '
        /^\[/   { sect = ($0 == want); next }
        sect && NF { print }
    ' "$LOCK"
}

cur_inputs="$(bindings_input_files | bindings_hash_list)"
cur_outputs="$(bindings_output_files | bindings_hash_list)"
rec_inputs="$(lock_block inputs)"
rec_outputs="$(lock_block outputs)"

rc=0

if [ "$cur_inputs" != "$rec_inputs" ]; then
    echo "❌ Binding drift: contract/core inputs changed without regenerated bindings." >&2
    echo "   (api/openapi.yaml, api/boundless.proto, or core/** differs from $LOCK)" >&2
    echo "   Run: bash scripts/generate-bindings.sh && commit the result." >&2
    diff <(printf '%s\n' "$rec_inputs") <(printf '%s\n' "$cur_inputs") | sed 's/^/     /' >&2 || true
    rc=1
fi

if [ "$cur_outputs" != "$rec_outputs" ]; then
    echo "❌ Binding drift: api/generated/** was hand-edited or is stale." >&2
    echo "   Generated files are never hand-edited. Run: bash scripts/generate-bindings.sh." >&2
    diff <(printf '%s\n' "$rec_outputs") <(printf '%s\n' "$cur_outputs") | sed 's/^/     /' >&2 || true
    rc=1
fi

if [ "$rc" -eq 0 ]; then
    n_in="$(printf '%s\n' "$cur_inputs" | grep -c .)"
    n_out="$(printf '%s\n' "$cur_outputs" | grep -c .)"
    echo "✓ bindings in sync (${n_in} inputs, ${n_out} generated files)."
fi
exit "$rc"
