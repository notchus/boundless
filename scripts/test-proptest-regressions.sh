#!/usr/bin/env bash
# Meta-test for the proptest seeds gate (scripts/check-proptest-regressions.sh).
# Proves the detectors bite + the prefix match is boundary-safe — a gate that can never fail is worthless.
#
# It exercises the gate's sourceable functions against synthetic inputs (and the live repo for discovery)
# WITHOUT mutating any git/Cargo state, so the tree stays pristine. The live git-state→detector seam (the
# `git ls-files` / `git check-ignore` calls in main) is exercised by the gate run in CI, not here.
set -euo pipefail
cd "$(dirname "$0")/.."

# Source the gate for its detectors. The gate's `if [[ BASH_SOURCE == $0 ]]` guard means sourcing defines
# functions/vars without running main (no git scan here).
# shellcheck source=scripts/check-proptest-regressions.sh
source scripts/check-proptest-regressions.sh

fail() { echo "❌ META-TEST FAILED: $1" >&2; exit 1; }

PREFIX="core/auth/proptest-regressions"

# 1. A tracked file under the regressions dir IS detected (the .gitkeep that makes the crate pass).
lsfiles_has_tracked_under "$PREFIX" <<<"core/auth/proptest-regressions/.gitkeep" \
    || fail "a tracked .gitkeep under the regressions dir was NOT detected"

# 2. A listing with NO file under the dir is NOT detected → the gate would flag that crate.
no_reg="$(printf '%s\n' "core/auth/Cargo.toml" "core/auth/src/lib.rs")"
if lsfiles_has_tracked_under "$PREFIX" <<<"$no_reg"; then
    fail "an absent regressions dir was wrongly treated as tracked"
fi

# 3. Boundary safety: a sibling dir sharing the prefix ('…-backup') must NOT count as the regressions dir.
if lsfiles_has_tracked_under "$PREFIX" <<<"core/auth/proptest-regressions-backup/seed.txt"; then
    fail "a '…-backup' sibling dir wrongly matched the regressions prefix"
fi

# 4. manifest_declares_proptest — the discovery predicate — must classify every TOML shape correctly.
#    POSITIVE (a crate that runs property tests → must be discovered):
manifest_declares_proptest <<<$'[dev-dependencies]\nproptest.workspace = true'        || fail "inline `proptest.workspace = true` not discovered"
manifest_declares_proptest <<<$'[dev-dependencies]\nproptest = "1"'                    || fail "inline `proptest = \"1\"` not discovered"
manifest_declares_proptest <<<$'[dev-dependencies.proptest]\nversion = "1"'            || fail "dotted-section `[dev-dependencies.proptest]` not discovered"
manifest_declares_proptest <<<$'[dev-dependencies] # host-only test deps\nproptest = "1"' || fail "commented dep-section header not discovered (F1 vector)"
manifest_declares_proptest <<<"[target.'cfg(unix)'.dev-dependencies]"$'\nproptest = "1"' || fail "target-specific dep table not discovered"
#    NEGATIVE (must NOT be discovered):
if manifest_declares_proptest <<<$'[workspace.dependencies]\nproptest = "1.11"';        then fail "workspace.dependencies registry wrongly discovered"; fi
if manifest_declares_proptest <<<$'[workspace.dependencies.proptest]\nversion = "1.11"'; then fail "dotted workspace.dependencies.proptest registry wrongly discovered"; fi
if manifest_declares_proptest <<<$'[dev-dependencies]\nproptest-derive = "0.5"';         then fail "`proptest-derive` (not proptest) wrongly discovered"; fi
if manifest_declares_proptest <<<$'[dev-dependencies]\nserde = "1"';                      then fail "a crate without proptest wrongly discovered"; fi

# 5. Live discovery sanity: the two known proptest crates are found and NOT the workspace root
#    (catches a real regression in the live find+predicate seam; the per-crate non-vacuity guard — the
#    gate itself only fails loud at ZERO crates, so this list must be kept current as proptest crates grow).
discovered="$(proptest_crate_dirs || true)"
grep -qx "core/auth" <<<"$discovered"   || fail "proptest_crate_dirs did not discover core/auth"
grep -qx "core/server" <<<"$discovered" || fail "proptest_crate_dirs did not discover core/server"
if grep -qx "core" <<<"$discovered"; then fail "proptest_crate_dirs wrongly discovered the workspace root (core/)"; fi

echo "✓ proptest-seeds meta-test passed (tracked→detected · absent→flagged · '-backup' boundary-safe · predicate covers inline/dotted/commented/target + excludes registry/derive · live discovery = auth+server, not root)."
