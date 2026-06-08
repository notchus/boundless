#!/usr/bin/env bash
# Meta-test for the wasm randomness gate (scripts/check-wasm-no-getrandom.sh).
# Proves the forbidden/allowed detectors actually bite — a gate that can never fail is worthless.
#
# It exercises has_forbidden_getrandom_edge() / has_allowed_getrandom_edge() against synthetic
# `cargo tree` renderings rather than mutating any Cargo manifest/lock (which would also perturb the
# binding-drift lock and need a `cargo update`). Touches no tracked state, so the tree stays pristine.
# The live cargo-query→detector seam is exercised by the gate run in CI, not here.
set -euo pipefail
cd "$(dirname "$0")/.."

# Source the gate for its detectors. The gate's `if [[ BASH_SOURCE == $0 ]]` guard means sourcing
# defines functions/vars without running main (no wasm build here).
# shellcheck source=scripts/check-wasm-no-getrandom.sh
source scripts/check-wasm-no-getrandom.sh

fail() { echo "❌ META-TEST FAILED: $1" >&2; exit 1; }

# 1. A forbidden 0.3.x node, INDENTED in a forward tree, MUST be flagged (the gate scans forward
#    trees, so the node sits under `── `, not at column 0).
indented_03="$(
    cat <<'EOF'
boundless-server-core v0.0.0
└── rand_core v0.3.1
    └── getrandom v0.3.4
EOF
)"
has_forbidden_getrandom_edge <<<"$indented_03" || fail "an indented getrandom 0.3.x node was NOT flagged"

# 2. Version-agnostic within the major: a *patch-bumped* 0.3.10 (the cargo-update regression this gate
#    exists to catch) MUST also be flagged — a version-pinned query would have missed it.
has_forbidden_getrandom_edge <<<"    └── getrandom v0.3.10" || fail "getrandom 0.3.10 (patch bump) was NOT flagged"

# 3. A clean tree carrying only the allowed 0.4.x edge MUST pass the forbidden check…
clean_04="$(
    cat <<'EOF'
boundless-crypto v0.0.0
├── dryoc v0.8.0
│   └── getrandom v0.4.2
└── getrandom v0.4.2 (*)
EOF
)"
if has_forbidden_getrandom_edge <<<"$clean_04"; then fail "a clean 0.4.x-only tree was wrongly flagged as forbidden"; fi

# 4. …and that same clean tree MUST satisfy the positive control (allowed 0.4.x present).
has_allowed_getrandom_edge <<<"$clean_04" || fail "the allowed 0.4.x edge was NOT detected by the positive control"

# 5. The positive control MUST fail (absent) on a getrandom-free tree (e.g. a randomness-free core
#    crate) — proving it would catch a vacuous pass.
if has_allowed_getrandom_edge <<<"boundless-sync v0.0.0"; then fail "positive control wrongly found a 0.4.x edge in a getrandom-free tree"; fi

# 6. Word-boundary guard: a package merely *ending* in 'getrandom' must NOT match.
if has_forbidden_getrandom_edge <<<"    └── notgetrandom v0.3.4"; then fail "a package ending in 'getrandom' wrongly matched"; fi

echo "✓ wasm-randomness meta-test passed (indented-0.3.x→flagged · 0.3.10→flagged · 0.4.x-only→clean · positive-control present/absent · word-boundary safe)."
