#!/usr/bin/env bash
# Boundless "reproducible proptest seeds" gate (constitution P9; spec 001 follow-up — DEFERRED T05 register).
#
# P9: "Property tests use proptest with seeds checked into the repo for reproducibility." proptest persists a
# failing case's seed to <crate>/proptest-regressions/<module>.txt and replays it first on later runs —
# committing that file is what makes a regression reproducible. But git does not track empty directories, so
# until a property first fails there is no proptest-regressions/ dir at all: a CI failure would write the seed
# to an UNtracked path (easy to lose), and nothing signals where seeds belong. This gate makes the guarantee
# mechanical — every crate declaring a `proptest` dev-dep must carry a git-tracked proptest-regressions/
# directory (a committed .gitkeep until the first real seed), and that path must not be gitignored (else a
# generated seed would be silently dropped).
#
# Crates are auto-discovered from core/**/Cargo.toml, so a new proptest-using crate is covered without editing
# this gate. Meta-tested by scripts/test-proptest-regressions.sh (which sources this file for the detectors).
set -euo pipefail

# Exit 0 iff the Cargo.toml on stdin declares `proptest` as a real per-crate
# (dev-/build-/target-)dependency. Section-aware (awk), covering both declaration forms:
#   • inline-table form — a `proptest…` key under a `[…dependencies]` table
#       (`proptest.workspace = true`, `proptest = "1"`, `proptest = { … }`); and
#   • dotted-section form — a `[…dependencies.proptest]` header.
# But NOT under the workspace dependency registry (`[workspace.dependencies]` /
# `[workspace.dependencies.proptest]`) — the workspace root merely *registers* the version there for
# members to inherit; the root itself runs no property tests. Header inline-comments and surrounding
# whitespace are tolerated, and `proptest-derive`/`proptest_*` keys do not false-match (the `=`/`.`
# suffix anchor). Stdin-driven so the meta-test can exercise it on synthetic manifests directly.
manifest_declares_proptest() {
    awk '
        function strip(s){ sub(/^[[:space:]]+/,"",s); sub(/[[:space:]]*#.*$/,"",s); sub(/[[:space:]]+$/,"",s); return s }
        /^[[:space:]]*\[/ {
            hdr = strip($0)
            if (hdr ~ /^\[workspace\.dependencies(\.|\])/) { in_dep = 0; next }   # the registry — exclude
            if (hdr ~ /dependencies\.proptest\]$/)         { found = 1; in_dep = 0; next }   # dotted-section form
            in_dep = (hdr ~ /dependencies\]$/)             # an inline-table dep section
            next
        }
        in_dep && /^[[:space:]]*proptest([[:space:]]*=|\.)/ { found = 1 }
        END { exit(found ? 0 : 1) }
    '
}

# Emit (on stdout) each core crate dir whose Cargo.toml declares proptest per the rule above. Auto-extends
# to any future proptest crate without editing this gate.
proptest_crate_dirs() {
    local manifest
    while IFS= read -r manifest; do
        if manifest_declares_proptest <"$manifest"; then
            dirname "$manifest"
        fi
    done < <(find core -name Cargo.toml -not -path '*/target/*')
}

# Read `git ls-files` output on stdin; exit 0 iff ≥1 tracked file sits under <prefix>/. The trailing slash
# anchors on the directory boundary, so a sibling like `…/proptest-regressions-backup/x` does NOT count as
# tracking `…/proptest-regressions`. Stdin-driven so the meta-test can exercise it on synthetic listings.
lsfiles_has_tracked_under() {
    local prefix="$1"
    grep -q "^${prefix}/"
}

main() {
    cd "$(dirname "${BASH_SOURCE[0]}")/.."

    local crates tracked dir regdir rc=0

    # `|| true` only swallows the bare pipefail exit when grep finds nothing — the non-vacuity guard right
    # below is the real fail-closed handling.
    crates="$(proptest_crate_dirs || true)"
    if [[ -z "$crates" ]]; then
        echo "❌ no core crate declares a proptest dev-dep — the discovery pattern may have broken; this gate" >&2
        echo "   would otherwise pass vacuously (P9). Re-examine scripts/check-proptest-regressions.sh." >&2
        exit 1
    fi

    tracked="$(git ls-files)"

    while IFS= read -r dir; do
        [[ -n "$dir" ]] || continue
        regdir="$dir/proptest-regressions"

        # (a) the regressions dir must be git-tracked (≥1 committed file: .gitkeep or a real seed).
        if ! lsfiles_has_tracked_under "$regdir" <<<"$tracked"; then
            echo "❌ $dir declares a proptest dev-dep but $regdir/ is not git-tracked — a failing property's" >&2
            echo "   seed would be written to an untracked path and lost. Commit $regdir/.gitkeep" >&2
            echo "   (P9: reproducible seeds checked into the repo)." >&2
            rc=1
            continue
        fi

        # (b) a future seed file under it must NOT be gitignored, else it is silently dropped.
        if git check-ignore -q "$regdir/sample.txt"; then
            echo "❌ $regdir/ is gitignored — a generated proptest seed would be silently dropped (P9)." >&2
            rc=1
        fi
    done <<<"$crates"

    [[ $rc -eq 0 ]] || exit 1
    echo "✓ proptest seeds gate passed — every proptest crate (${crates//$'\n'/ }) has a tracked, un-ignored proptest-regressions/ (P9)."
}

# Run only when executed directly; `source` (the meta-test) gets the functions without running main.
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
