#!/usr/bin/env bash
# Boundless "scaffold can never be silently deployed" gate (security-auditor F1; spec 001 T07-shell-B).
#
# The deployable Worker (`server/`) currently composes its real core `AuthService` over an in-memory
# `ScaffoldStore` — a HARDCODED dev HMAC key (`[0x7b; 32]`) + one seeded demo member — which must NEVER
# reach production. That scaffold is build-gated behind the non-default `scaffold` cargo feature: the
# local/test path opts in (`worker-build --release --features scaffold`, server/package.json), while the
# deploy path (`wrangler deploy` → wrangler.toml [build] = `worker-build --release`) stays featureless
# and hits a `compile_error!` (server/src/lib.rs). This gate makes that fail-closed property MECHANICAL
# (sec-audit F1 asked for enforcement, not a guard that could silently regress): it proves a featureless
# wasm build of the Worker actually fails, AND fails *via the guard* (not some unrelated breakage).
#
# The POSITIVE path (the with-`--features scaffold` build + the miniflare suite) is proven by `pnpm test`
# in the CI `worker` job, which runs immediately before this gate — so this gate need only assert the
# negative (deploy) path. It is self-contained for local runs too (it just compiles slower without the
# warm target/ cache). No meta-test: the only logic here is exit-code + sentinel, and the real thing
# under test (the `compile_error!`) is exercised directly.
set -euo pipefail

WASM=wasm32-unknown-unknown
# A stable substring of the `compile_error!` in server/src/lib.rs. If that message is reworded, update
# this in lock-step (the grep below will fail loudly until you do — that is the gate biting on drift).
SENTINEL="boundless-worker has no production store yet"

main() {
    cd "$(dirname "${BASH_SOURCE[0]}")/../server"

    echo "→ boundless-worker: featureless wasm build must FAIL CLOSED (no store wired = no deploy)"

    # Build the deploy artifact's shape: a featureless wasm build (exactly what `worker-build --release`
    # hands to cargo). We WANT this to fail. Capture stdout+stderr; the `if` condition keeps `set -e`
    # from exiting on the expected non-zero.
    local out
    if out="$(cargo build --locked -p boundless-worker --lib --release --target "$WASM" 2>&1)"; then
        echo "❌ the featureless wasm build SUCCEEDED — the fail-closed deploy guard is missing." >&2
        echo "   A production \`wrangler deploy\` could now ship the scaffold (hardcoded dev HMAC key +" >&2
        echo "   seeded demo member). Restore the \`compile_error!\` in server/src/lib.rs (gated on" >&2
        echo "   \`not(feature = \"scaffold\")\`) — security-auditor F1, DEFERRED.md → T07-shell-B." >&2
        exit 1
    fi

    # It failed — confirm it failed *because of the guard*, not some unrelated build breakage (e.g. a
    # missing wasm target or a dep error), which would make this gate pass vacuously.
    if ! grep -qF "$SENTINEL" <<<"$out"; then
        echo "❌ the featureless wasm build failed, but NOT via the deploy guard (sentinel not found)." >&2
        echo "   Expected the \`compile_error!\` naming: \"$SENTINEL\". The build broke for another" >&2
        echo "   reason — investigate before trusting the fail-closed property. Build output:" >&2
        echo "$out" >&2
        exit 1
    fi

    echo "✓ worker deploy guard passed — a featureless (deploy) wasm build fails closed via the"
    echo "  compile_error! in server/src/lib.rs; the scaffold cannot be silently deployed (F1)."
}

# Run only when executed directly; `source` gets the function without running main.
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
