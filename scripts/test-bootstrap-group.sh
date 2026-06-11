#!/usr/bin/env bash
# Meta-test for scripts/bootstrap-group.sh + examples/bootstrap_group_pg.rs — spec 008 (the
# test-binding-drift.sh idiom). Account-free: provisions a THROWAWAY local DB, applies the migrations,
# then drives the REAL bootstrap wrapper and proves the operator-facing invariants:
#   * a fresh Group is bootstrapped — a `delegated_keys` row appears, length = NONCE+MAC+KEY (72 bytes),
#     and the wrapper reports "bootstrapped" (the example's self-check unwrap with the KEK already passed,
#     else it would have panicked → non-zero exit → the wrapper's error path);
#   * IDEMPOTENT / NEVER-OVERWRITE — a second run reports "already bootstrapped — left untouched" and the
#     `wrapped_key` bytes are BYTE-IDENTICAL (the example's `ON CONFLICT DO NOTHING` did not mint a new
#     key over the old one — overwriting would orphan all PII encrypted under it);
#   * the groups row exists (the FK prerequisite).
#
# Self-skips when no Postgres is reachable (mirrors the store suite). Local:
#   PSQL="psql" WORKER_TEST_SUPERUSER_URL=postgres://notch@localhost:5432/postgres \
#   bash scripts/test-bootstrap-group.sh
set -euo pipefail
cd "$(dirname "$0")/.."

PSQL="${PSQL:-psql}"
SU_URL="${WORKER_TEST_SUPERUSER_URL:-postgres://postgres:postgres@localhost:55432/boundless_test}"
BOOT_DB="boundless_bootstrap_test"     # throwaway; this script DROP/CREATEs it
MIG_DIR="server/migrations"
# Fixed test values. The KEK is a LOCAL TEST value (not a secret); the group id/name are arbitrary.
KEK_HEX="cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
GID="40000000-0000-0000-0000-000000000000"
GNAME="Bootstrap Test Group"

fail() { echo "❌ META-TEST FAILED: $1" >&2; exit 1; }
q() { printf '%s' "$($PSQL "$1" -tAc "$2")" | tr -d '[:space:]'; }

# --- self-skip when no PG is reachable --------------------------------------------------------
if ! $PSQL "$SU_URL" -tAc 'SELECT 1' >/dev/null 2>&1; then
  echo "ℹ skipping bootstrap-group meta-test (no Postgres at WORKER_TEST_SUPERUSER_URL; set it + PSQL to run)" >&2
  exit 0
fi

# --- 1. fresh throwaway DB + migrations (as the superuser; tables owned by it, BYPASSRLS) ----------
SU_PREFIX="${SU_URL%/*}"                                  # …@host:port (drop /db)
SU_TARGET="${SU_PREFIX}/${BOOT_DB}"                       # superuser → throwaway DB (the "owner" we bootstrap as)
$PSQL "$SU_URL" -v ON_ERROR_STOP=1 -tAc "DROP DATABASE IF EXISTS ${BOOT_DB} WITH (FORCE)" >/dev/null
$PSQL "$SU_URL" -v ON_ERROR_STOP=1 -tAc "CREATE DATABASE ${BOOT_DB}" >/dev/null
shopt -s nullglob
for f in "$MIG_DIR"/*.up.sql; do
  $PSQL "$SU_TARGET" -v ON_ERROR_STOP=1 -q --single-transaction < "$f"
done
[ "$(q "$SU_TARGET" "SELECT to_regclass('public.delegated_keys') IS NOT NULL")" = "t" ] \
  || fail "setup: delegated_keys table missing after migrations"

# --- 2. first bootstrap — must mint a key (the SU_TARGET owner is BYPASSRLS, so the insert lands) ---
err1="$(mktemp)"; trap 'rm -f "$err1" "${err2:-}"' EXIT
BOOTSTRAP_KEK_HEX="$KEK_HEX" bash scripts/bootstrap-group.sh "$SU_TARGET" "$GID" "$GNAME" 2>"$err1" \
  || { cat "$err1" >&2; fail "first bootstrap exited non-zero"; }
grep -q "bootstrapped" "$err1" || { cat "$err1" >&2; fail "first run did not report a bootstrap"; }
grep -q "already" "$err1" && { cat "$err1" >&2; fail "first run reported 'already' on an empty DB"; }

[ "$(q "$SU_TARGET" "SELECT count(*) FROM groups WHERE id='${GID}'")" = "1" ]          || fail "groups row was not created"
[ "$(q "$SU_TARGET" "SELECT count(*) FROM delegated_keys WHERE group_id='${GID}'")" = "1" ] || fail "delegated_keys row was not created"
# Structural: nonce(24) ‖ ciphertext(MAC 16 + key 32) = 72 bytes (core/crypto wrap_group_key).
[ "$(q "$SU_TARGET" "SELECT octet_length(wrapped_key) FROM delegated_keys WHERE group_id='${GID}'")" = "72" ] \
  || fail "wrapped_key has the wrong length (expected 72 = NONCE+MAC+KEY)"
key1="$(q "$SU_TARGET" "SELECT encode(wrapped_key,'hex') FROM delegated_keys WHERE group_id='${GID}'")"
[ -n "$key1" ] || fail "could not read the wrapped_key back"

# --- 3. second bootstrap — IDEMPOTENT, NEVER overwrites --------------------------------------------
err2="$(mktemp)"
BOOTSTRAP_KEK_HEX="$KEK_HEX" bash scripts/bootstrap-group.sh "$SU_TARGET" "$GID" "$GNAME" 2>"$err2" \
  || { cat "$err2" >&2; fail "second bootstrap exited non-zero"; }
grep -q "already bootstrapped" "$err2" || { cat "$err2" >&2; fail "second run did not report 'already bootstrapped'"; }
[ "$(q "$SU_TARGET" "SELECT count(*) FROM delegated_keys WHERE group_id='${GID}'")" = "1" ] || fail "second run created a duplicate/extra key row"
key2="$(q "$SU_TARGET" "SELECT encode(wrapped_key,'hex') FROM delegated_keys WHERE group_id='${GID}'")"
[ "$key1" = "$key2" ] || fail "the wrapped_key CHANGED across runs — bootstrap OVERWROTE an existing key (would orphan all PII)"

# --- 4. cleanup -----------------------------------------------------------------------------------
$PSQL "$SU_URL" -v ON_ERROR_STOP=1 -tAc "DROP DATABASE IF EXISTS ${BOOT_DB} WITH (FORCE)" >/dev/null

echo "✓ bootstrap-group meta-test passed (mints a 72-byte KEK-wrapped key · groups+delegated_keys rows · idempotent · NEVER overwrites an existing key)."
