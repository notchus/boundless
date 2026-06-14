#!/usr/bin/env bash
# Meta-test for scripts/seed-admin-invite.sh + examples/seed_admin_invite_pg.rs — spec 009 T10, AC7
# (the test-bootstrap-group.sh idiom). Account-free: provisions a THROWAWAY local DB, applies the
# migrations + a `groups` row, then drives the REAL seed wrapper and proves the operator-facing
# invariants:
#   * CREATE — a fresh run mints exactly ONE pending Admin (role admin, NO PII: null name/address/phone)
#     and exactly ONE live invitation whose token_hash is the 32-byte keyed hash (not the token);
#   * R20 — the minted token appears ONLY on the wrapper's stdout, NEVER in its stderr log lines;
#   * IDEMPOTENT RE-INVITE (R19) — a second run WITH the minted admin-id supersedes the prior invite
#     and mints a fresh one: still exactly ONE live invitation (one_live_per_admin upheld), two rows
#     total (one consumed), a DIFFERENT token, and NO second admin.
#
# Self-skips when no Postgres is reachable (mirrors the store suite). Local:
#   PSQL="psql" WORKER_TEST_SUPERUSER_URL=postgres://notch@localhost:5432/postgres \
#   bash scripts/test-seed-admin-invite.sh
set -euo pipefail
cd "$(dirname "$0")/.."

PSQL="${PSQL:-psql}"
SU_URL="${WORKER_TEST_SUPERUSER_URL:-postgres://postgres:postgres@localhost:55432/boundless_test}"
SEED_DB="boundless_seed_admin_test"     # throwaway; this script DROP/CREATEs it
MIG_DIR="server/migrations"
# Fixed test values. The HMAC key is a LOCAL TEST value (not a secret); the group id/name are arbitrary.
HMAC_HEX="abababababababababababababababababababababababababababababababab"
GID="41000000-0000-0000-0000-000000000000"
GNAME="Seed-Admin Test Group"

fail() { echo "❌ META-TEST FAILED: $1" >&2; exit 1; }
q() { printf '%s' "$($PSQL "$1" -tAc "$2")" | tr -d '[:space:]'; }

# Defense-in-depth (P2): the owner URL's userinfo (`user[:pass]`) must never reach a stderr log line —
# the wrapper logs only a redacted `scheme://host/db`. We assert on the WHOLE userinfo segment (not the
# bare password): the colon-joined `user:pass` is a structured token the redacted url can't contain, so
# this catches a genuine leak WITHOUT false-firing on the `postgres://` scheme word or the db name (a
# bare-password `grep` would false-fail in CI, where the common password `postgres` is a scheme substring).
# `CREDS` is set after SEED_URL is built (below).
CREDS=""
assert_no_userinfo() { # $1=label $2=errfile
  [ -z "$CREDS" ] && return 0
  ! grep -qF "$CREDS" "$2" || fail "$1: the owner URL userinfo leaked into stderr (P2)"
}

# --- self-skip when no PG is reachable --------------------------------------------------------
if ! $PSQL "$SU_URL" -tAc 'SELECT 1' >/dev/null 2>&1; then
  echo "ℹ skipping seed-admin-invite meta-test (no Postgres at WORKER_TEST_SUPERUSER_URL; set it + PSQL to run)" >&2
  exit 0
fi

# --- 1. fresh throwaway DB + migrations + a groups row (as the superuser; BYPASSRLS owner) ----------
SU_PREFIX="${SU_URL%/*}"                                  # …@host:port (drop /db)
SU_TARGET="${SU_PREFIX}/${SEED_DB}"                       # superuser → throwaway DB (the "owner" we seed as)
# The seed connects with a real TLS connector (Neon requires TLS). The LOCAL test PG may not offer TLS,
# so force `sslmode=disable` (plaintext) for it. (The operator's Neon URL carries `?sslmode=require`.)
SEED_URL="${SU_TARGET}?sslmode=disable"
# The userinfo segment (user[:pass]) of the seed connection URL, for the redaction self-check (above).
case "$SEED_URL" in *://*@*) CREDS="${SEED_URL#*://}"; CREDS="${CREDS%@*}" ;; esac
$PSQL "$SU_URL" -v ON_ERROR_STOP=1 -tAc "DROP DATABASE IF EXISTS ${SEED_DB} WITH (FORCE)" >/dev/null
$PSQL "$SU_URL" -v ON_ERROR_STOP=1 -tAc "CREATE DATABASE ${SEED_DB}" >/dev/null
shopt -s nullglob
for f in "$MIG_DIR"/*.up.sql; do
  $PSQL "$SU_TARGET" -v ON_ERROR_STOP=1 -q --single-transaction < "$f"
done
# The Group must exist first (members.group_id → groups, the FK the seed relies on — bootstrap-group.sh
# creates it in the real flow). No delegated_keys/key needed: admins carry no encrypted PII.
$PSQL "$SU_TARGET" -v ON_ERROR_STOP=1 -tAc \
  "INSERT INTO groups (id, name) VALUES ('${GID}', '${GNAME}') ON CONFLICT (id) DO NOTHING" >/dev/null
[ "$(q "$SU_TARGET" "SELECT count(*) FROM groups WHERE id='${GID}'")" = "1" ] || fail "setup: groups row missing"

out1="$(mktemp)"; err1="$(mktemp)"; out2="$(mktemp)"; err2="$(mktemp)"
trap 'rm -f "$out1" "$err1" "$out2" "$err2"' EXIT

# --- 2. CREATE — mints one null-PII pending Admin + one live invitation -----------------------------
SEED_OWNER_URL="$SEED_URL" SEED_HMAC_KEY_HEX="$HMAC_HEX" \
  bash scripts/seed-admin-invite.sh "$GID" 1>"$out1" 2>"$err1" \
  || { cat "$err1" >&2; fail "create run exited non-zero"; }

AID="$(sed -n 's/^admin_id=//p' "$out1" | tr -d '[:space:]')"
TOKEN1="$(sed -n 's/^token=//p' "$out1" | tr -d '[:space:]')"
[ -n "$AID" ]    || { cat "$out1" >&2; fail "create did not print admin_id"; }
[ -n "$TOKEN1" ] || { cat "$out1" >&2; fail "create did not print token"; }
[ "${#TOKEN1}" -eq 64 ] || fail "token is not 64 hex chars (got ${#TOKEN1})"
case "$TOKEN1" in *[!0-9a-fA-F]*) fail "token is not hex" ;; esac

# A null-PII pending Admin: role admin, NO name/address/phone (Admins authenticate via WebAuthn).
[ "$(q "$SU_TARGET" "SELECT count(*) FROM members WHERE id='${AID}' AND roles @> ARRAY['admin']::member_role[] AND name_encrypted IS NULL AND address_encrypted IS NULL AND phone_lookup_hash IS NULL")" = "1" ] \
  || fail "expected exactly one null-PII admin member"
# Exactly one live invitation; its token_hash is the 32-byte keyed hash (the at-rest form, never the token).
[ "$(q "$SU_TARGET" "SELECT count(*) FROM admin_invitations WHERE admin_id='${AID}' AND consumed_at IS NULL")" = "1" ] \
  || fail "expected exactly one live invitation after create"
[ "$(q "$SU_TARGET" "SELECT octet_length(token_hash) FROM admin_invitations WHERE admin_id='${AID}' AND consumed_at IS NULL")" = "32" ] \
  || fail "token_hash is not 32 bytes (the keyed HMAC)"
# The plaintext token is NOT what is stored (only its hash) — the stored hash hex must differ from the token.
[ "$(q "$SU_TARGET" "SELECT count(*) FROM admin_invitations WHERE admin_id='${AID}' AND encode(token_hash,'hex')='${TOKEN1}'")" = "0" ] \
  || fail "the plaintext token was stored verbatim — it must be hashed (P2/R20)"

# R20 — the token must appear ONLY on stdout, never in a stderr log line.
if grep -qF "$TOKEN1" "$err1"; then cat "$err1" >&2; fail "the minted token LEAKED into the wrapper's stderr log (R20)"; fi
# Defense-in-depth: the owner URL's userinfo must not reach stderr either (the redaction).
assert_no_userinfo "create" "$err1"

# --- 3. RE-INVITE (idempotent, R19) — supersede-then-insert, exactly one live ----------------------
SEED_OWNER_URL="$SEED_URL" SEED_HMAC_KEY_HEX="$HMAC_HEX" \
  bash scripts/seed-admin-invite.sh "$GID" "$AID" 1>"$out2" 2>"$err2" \
  || { cat "$err2" >&2; fail "re-invite run exited non-zero"; }
grep -q "re-invited" "$err2" || { cat "$err2" >&2; fail "re-invite run did not report a re-invite"; }

TOKEN2="$(sed -n 's/^token=//p' "$out2" | tr -d '[:space:]')"
[ -n "$TOKEN2" ] || fail "re-invite did not print a fresh token"
[ "$TOKEN2" != "$TOKEN1" ] || fail "re-invite reused the same token (a fresh one must be minted)"
if grep -qF "$TOKEN2" "$err2"; then fail "the re-invite token LEAKED into stderr (R20)"; fi
assert_no_userinfo "re-invite" "$err2"

# Still ONE admin, ONE live invite (one_live_per_admin upheld), TWO rows total (one consumed).
[ "$(q "$SU_TARGET" "SELECT count(*) FROM members WHERE roles @> ARRAY['admin']::member_role[]")" = "1" ] \
  || fail "re-invite created a SECOND admin (it must re-invite the SAME one)"
[ "$(q "$SU_TARGET" "SELECT count(*) FROM admin_invitations WHERE admin_id='${AID}' AND consumed_at IS NULL")" = "1" ] \
  || fail "expected exactly one LIVE invitation after re-invite (one_live_per_admin)"
[ "$(q "$SU_TARGET" "SELECT count(*) FROM admin_invitations WHERE admin_id='${AID}'")" = "2" ] \
  || fail "expected two total invitation rows after re-invite (one consumed, one live)"

# --- 4. a missing admin-id is a clean error, not a stray mint --------------------------------------
PHANTOM="4100dead-0000-0000-0000-000000000000"
if SEED_OWNER_URL="$SEED_URL" SEED_HMAC_KEY_HEX="$HMAC_HEX" \
     bash scripts/seed-admin-invite.sh "$GID" "$PHANTOM" >/dev/null 2>"$err2"; then
  fail "re-inviting a non-existent admin-id should fail (exit non-zero), not succeed"
fi
grep -q "no pending Admin" "$err2" || { cat "$err2" >&2; fail "phantom re-invite did not report a clear 'no pending Admin' error"; }
[ "$(q "$SU_TARGET" "SELECT count(*) FROM admin_invitations WHERE admin_id='${PHANTOM}'")" = "0" ] \
  || fail "a phantom re-invite minted a stray invitation"

# --- 5. cleanup -----------------------------------------------------------------------------------
$PSQL "$SU_URL" -v ON_ERROR_STOP=1 -tAc "DROP DATABASE IF EXISTS ${SEED_DB} WITH (FORCE)" >/dev/null

echo "✓ seed-admin-invite meta-test passed (null-PII admin + one live invite · token only on stdout, R20 · idempotent re-invite keeps one_live_per_admin · phantom re-invite is a clean error)."
