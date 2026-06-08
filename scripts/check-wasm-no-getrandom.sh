#!/usr/bin/env bash
# Boundless "no ambient randomness in core" wasm gate (ADR-0021; sec-audit F6; spec 001 follow-up).
#
# The privacy/security model requires randomness to be *injected* into the core (the
# `RngSecretSource<R>` discipline) — never pulled ambiently. The concrete failure mode is
# `getrandom 0.3.x` (the OS-randomness backend) reaching the **production** (non-dev) wasm path of a
# core crate. The *allowed* edge is `getrandom 0.4.x`, which is `wasm_js`-shimmed and deterministic
# (crypto's explicit shim + dryoc→rand). Both versions coexist in Cargo.lock (0.3.x arrives only via
# dev-deps such as proptest), so the separation is real but un-gated: a `cargo update` or a dep
# flipping `rand_core/os_rng` could silently route 0.3.x onto the prod path. This gate makes the
# invariant mechanical instead of a manual check.
#
# Coverage: F6 names boundless-server-core + boundless-crypto. server-core sits atop the core stack,
# so its non-dev tree transitively covers boundless-domain / -auth / -crypto; crypto is also audited
# directly (it carries the explicit `wasm_js` shim and is a separate FFI consumer). The other
# wasm-relevant core crates (ffi-wasm — the literal browser artifact — sync, logging) and the server/
# workspace's deployed Worker are dependency-free / out of scope today and are tracked in DEFERRED.md
# (extend CRATES when they gain a getrandom edge).
#
# Meta-tested by scripts/test-wasm-no-getrandom.sh (which sources this file for the detectors below).
set -euo pipefail

CRATES=(boundless-server-core boundless-crypto)
WASM=wasm32-unknown-unknown

# Detectors read a `cargo tree` rendering on stdin and exit 0 (match) iff a getrandom node of the
# given major is present. The `(^| )` prefix anchors on the column-0 root OR the space after a tree
# branch (`──␣`), so a hypothetical package merely *ending* in "getrandom" can't match. They are
# version-agnostic *within* the major — any 0.3.x (forbidden) / any 0.4.x (allowed) — so a `cargo
# update` patch bump (e.g. 0.3.4 → 0.3.10) cannot slip past a version-pinned query.
has_forbidden_getrandom_edge() { grep -Eq '(^| )getrandom v0\.3\.'; }
has_allowed_getrandom_edge() { grep -Eq '(^| )getrandom v0\.4\.'; }

main() {
    cd "$(dirname "${BASH_SOURCE[0]}")/../core"

    local crate tree
    for crate in "${CRATES[@]}"; do
        echo "→ $crate: wasm32 build + getrandom-edge audit"

        # (a) Build for wasm32 — proves wasm-cleanliness and (since a bad crate name errors here)
        # serves as the non-vacuity guard for the crate identifier itself.
        cargo build --locked --target "$WASM" -p "$crate"

        # (b) Inspect the crate's PRODUCTION (non-dev) wasm dependency tree. Forward (not `-i`) and
        # version-agnostic, so a patch bump of the backend can't slip past; no `|| true`, so a genuine
        # resolve error fails the gate (fail-closed). dev-deps (proptest's getrandom 0.3.x, rand_chacha)
        # are excluded by `-e no-dev` and must NOT trip the gate.
        tree="$(cargo tree --locked -p "$crate" -e no-dev --target "$WASM")"

        if has_forbidden_getrandom_edge <<<"$tree"; then
            echo "❌ getrandom 0.3.x is on $crate's production wasm path — ambient OS randomness in core (ADR-0021 / F6):" >&2
            grep -E '(^| )getrandom v0\.3\.' <<<"$tree" >&2
            exit 1
        fi

        # Positive control (non-vacuity): the allowed 0.4.x (wasm_js-shimmed) edge MUST be present, so
        # the absence check above can't pass vacuously / the probe can't have silently broken. If it
        # ever vanishes, fail loud so a human re-examines this gate against the changed dep graph.
        if ! has_allowed_getrandom_edge <<<"$tree"; then
            echo "❌ expected the allowed getrandom 0.4.x (wasm_js-shimmed) edge on $crate — found none." >&2
            echo "   The dependency graph changed; re-examine this gate (the 0.3.x absence check may now be vacuous)." >&2
            exit 1
        fi
    done

    echo "✓ wasm randomness gate passed — no getrandom 0.3.x on the production wasm path of ${CRATES[*]} (only the allowed 0.4.x wasm_js shim)."
}

# Run only when executed directly; `source` (the meta-test) gets the functions without running main.
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
