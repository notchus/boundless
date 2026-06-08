#!/usr/bin/env bash
# Meta-test for scripts/provision-neon.sh — spec 001 T07-shell-B (the test-binding-drift.sh idiom).
#
# Account-free: provisions a THROWAWAY local database exactly as the deploy runbook provisions a real
# Neon DB, then — connecting as the REAL non-superuser `boundless_app` LOGIN role (not `SET ROLE` on a
# superuser connection, the way server/store/tests/common/mod.rs does) — proves the lock-down holds.
# This is a STRICTLY STRONGER RLS proof than the Rust harness: it is the account-free analog of the
# sec-audit F5 "deployed-edge cross-tenant smoke as the real app role" (DEFERRED.md → T07-shell-B).
# Fidelity note: locally the OWNER is a true superuser (the seed bypasses RLS via superuser semantics),
# whereas Neon's real owner `neondb_owner` is a non-superuser WITH BYPASSRLS — a different bypass path.
# The load-bearing assertions are all AS the `boundless_app` app role; the production-faithful proof is
# still the live deployed-edge cross-tenant smoke (DEFERRED.md → T07-shell-B, sec-audit F5).
#
# Asserts, as boundless_app over the freshly-provisioned `public`:
#   role attrs (NOSUPERUSER/NOBYPASSRLS/LOGIN) · 8 tables · single uniform table owner (catches a Neon
#   ownership split a local superuser would mask) · the literal `ensure_least_privilege` SQL returns
#   both-false (+ a superuser returns is_super=true — the guard's negative) · current_group_id() executes
#   (PUBLIC-EXECUTE) · DML works under the tenant GUC · RLS isolates cross-tenant reads · fail-closed
#   (no GUC → zero rows) · the printed connection string has the expected shape.
#
# Self-skips when no Postgres is reachable (mirrors the store suite's url_or_skip!). CI `worker` job runs
# it (has postgres:18) BEFORE setup-worker-test-db.sh, with BOUNDLESS_APP_DB_PASSWORD=boundless_app —
# `boundless_app` is a CLUSTER-GLOBAL role shared with that DB, so pinning the password keeps the two
# order-independent. Local:
#   PSQL="docker exec -i boundless-postgres psql" \
#   WORKER_TEST_SUPERUSER_URL=postgres://postgres:postgres@localhost:5432/boundless_test \
#   bash scripts/test-provision-neon.sh
set -euo pipefail
cd "$(dirname "$0")/.."

PSQL="${PSQL:-psql}"
# Superuser URL (also the maintenance connection that DROP/CREATEs the throwaway DB). Local default
# = the dev container on :55432; CI overrides to its service (:5432).
SU_URL="${WORKER_TEST_SUPERUSER_URL:-postgres://postgres:postgres@localhost:55432/boundless_test}"
PROV_DB="boundless_provision_test"   # name matches provision-neon.sh's nothing-destructive; safe throwaway
APP_PW="boundless_app"               # pin (shared cluster-global role — see header)

fail() { echo "❌ META-TEST FAILED: $1" >&2; exit 1; }
# A scalar query, whitespace-trimmed. $1=url $2=sql
q() { printf '%s' "$($PSQL "$1" -tAc "$2")" | tr -d '[:space:]'; }

# --- self-skip when no PG is reachable --------------------------------------------------------
if ! $PSQL "$SU_URL" -tAc 'SELECT 1' >/dev/null 2>&1; then
  echo "ℹ skipping provision-neon meta-test (no Postgres at WORKER_TEST_SUPERUSER_URL; set it + PSQL to run)" >&2
  exit 0
fi

# Derive the throwaway-DB URLs from SU_URL (swap the trailing /db; for the app URL also swap userinfo).
SU_PREFIX="${SU_URL%/*}"                                   # …@host:port  (drop /db)
OWNER_TARGET="${SU_PREFIX}/${PROV_DB}"                     # superuser → throwaway DB
APP_TARGET="$(printf '%s' "$OWNER_TARGET" | sed -E "s#^(postgres(ql)?://)[^/]*@#\1boundless_app:${APP_PW}@#")"

# 8 tables the migrations create.
TABLES="groups members onboarding_codes recovery_codes device_tokens sessions admin_webauthn_credentials admin_invitations"
in_list="$(printf "'%s'," $TABLES)"; in_list="${in_list%,}"

# Fixed test ids.
GA="10000000-0000-0000-0000-000000000000"; MA="1a000000-0000-0000-0000-000000000000"
GB="20000000-0000-0000-0000-000000000000"; MB="2b000000-0000-0000-0000-000000000000"
GC="30000000-0000-0000-0000-000000000000"   # the DML-probe group

# --- 1. fresh throwaway DB (DROP/CREATE in autocommit — can't be in a txn; WITH FORCE = PG13+) ----
$PSQL "$SU_URL" -v ON_ERROR_STOP=1 -tAc "DROP DATABASE IF EXISTS ${PROV_DB} WITH (FORCE)" >/dev/null
$PSQL "$SU_URL" -v ON_ERROR_STOP=1 -tAc "CREATE DATABASE ${PROV_DB}" >/dev/null

# --- 2. provision it exactly as the runbook provisions Neon; capture the printed conn string -------
CONN="$(BOUNDLESS_APP_DB_PASSWORD="$APP_PW" PSQL="$PSQL" bash scripts/provision-neon.sh "$OWNER_TARGET" | tail -1)"

# --- 3. connection-string shape (pure string assert; bare — sslmode is the wrangler --sslmode flag) ---
echo "$CONN" | grep -Eq "^postgres(ql)?://boundless_app:${APP_PW}@.*/${PROV_DB}$" \
  || fail "printed conn string has the wrong shape: ${CONN}"

# --- 3b. negative: a non-URL/SQL-safe app password is REJECTED before any DB work (closes the sed-`&`
# owner-credential leak). provision-neon.sh validates the password first, so this never touches the DB. --
if BOUNDLESS_APP_DB_PASSWORD='a&b' PSQL="$PSQL" bash scripts/provision-neon.sh "$OWNER_TARGET" >/dev/null 2>&1; then
  fail "provision-neon.sh must reject a non-URL/SQL-safe BOUNDLESS_APP_DB_PASSWORD (e.g. 'a&b')"
fi

# --- 4. role attributes (NOSUPERUSER / NOBYPASSRLS / LOGIN) ---------------------------------------
[ "$(q "$OWNER_TARGET" "SELECT rolsuper      FROM pg_roles WHERE rolname='boundless_app'")" = "f" ] || fail "boundless_app must be NOSUPERUSER"
[ "$(q "$OWNER_TARGET" "SELECT rolbypassrls  FROM pg_roles WHERE rolname='boundless_app'")" = "f" ] || fail "boundless_app must be NOBYPASSRLS"
[ "$(q "$OWNER_TARGET" "SELECT rolcanlogin   FROM pg_roles WHERE rolname='boundless_app'")" = "t" ] || fail "boundless_app must be LOGIN"

# --- 5. schema: 8 tables, single uniform owner (Neon ownership-split guard) ------------------------
[ "$(q "$OWNER_TARGET" "SELECT count(*) FROM pg_tables WHERE schemaname='public' AND tablename IN (${in_list})")" = "8" ] \
  || fail "expected 8 provisioned tables"
[ "$(q "$OWNER_TARGET" "SELECT count(DISTINCT tableowner) FROM pg_tables WHERE schemaname='public' AND tablename IN (${in_list})")" = "1" ] \
  || fail "tables must share ONE owner (a split would let ON ALL TABLES silently under-grant on Neon)"

# --- 6. the W2 guard, as the app role (load-bearing) + its negative (superuser) --------------------
[ "$(q "$APP_TARGET"   "SELECT current_setting('is_superuser')::bool")" = "f" ] || fail "ensure_least_privilege: app role must be is_superuser=false"
[ "$(q "$APP_TARGET"   "SELECT COALESCE((SELECT rolbypassrls FROM pg_roles WHERE rolname=current_user),false)")" = "f" ] || fail "ensure_least_privilege: app role must be rolbypassrls=false"
[ "$(q "$OWNER_TARGET" "SELECT current_setting('is_superuser')::bool")" = "t" ] || fail "negative check: the superuser owner must report is_superuser=true (guard would reject it)"

# --- 7. PUBLIC-EXECUTE: current_group_id() runs as the app role (errors if EXECUTE were revoked) ----
[ "$(q "$APP_TARGET" "SELECT current_group_id() IS NULL")" = "t" ] || fail "app role must be able to SELECT current_group_id()"

# --- 8. seed two tenants as superuser (bypasses RLS), then prove isolation AS the app role ---------
$PSQL "$OWNER_TARGET" -v ON_ERROR_STOP=1 -q <<SQL
INSERT INTO groups (id, name) VALUES ('${GA}','A'), ('${GB}','B');
INSERT INTO members (id, group_id, roles, phone_lookup_hash) VALUES
  ('${MA}','${GA}','{rider}'::member_role[], '\x01'),
  ('${MB}','${GB}','{rider}'::member_role[], '\x02');
SQL

# Cross-tenant isolation: scoped to A, A's member is visible and B's is not (mirrors
# server/store/tests/integration.rs::rls_isolates_reads_by_tenant, but as the real LOGIN role).
# `if !` (not a bare `iso=$(…)`) so a probe that *errors* on a regression (e.g. a missing SELECT grant)
# fires the friendly fail() instead of `set -e` killing the script before the `||fail` line is reached.
if ! iso="$($PSQL "$APP_TARGET" -tA -v ON_ERROR_STOP=1 <<SQL
BEGIN;
SELECT set_config('app.current_group_id', '${GA}', true);
SELECT 'A=' || count(*) FROM members WHERE id='${MA}';
SELECT 'B=' || count(*) FROM members WHERE id='${MB}';
COMMIT;
SQL
)"; then fail "RLS isolation probe errored as the app role (missing SELECT grant / RLS regression?)"; fi
echo "$iso" | grep -q '^A=1$' || fail "tenant A must see its own member under the GUC"
echo "$iso" | grep -q '^B=0$' || fail "tenant A must NOT see tenant B's member (RLS isolation)"

# DML as the app role: under GUC=C, INSERT a group (WITH CHECK id=current_group_id()) then read it back.
if ! dml="$($PSQL "$APP_TARGET" -tA -v ON_ERROR_STOP=1 <<SQL
BEGIN;
SELECT set_config('app.current_group_id', '${GC}', true);
INSERT INTO groups (id, name) VALUES ('${GC}', 'dml probe');
SELECT 'C=' || count(*) FROM groups WHERE id='${GC}';
COMMIT;
SQL
)"; then fail "DML probe errored as the app role (missing INSERT grant?)"; fi
echo "$dml" | grep -q '^C=1$' || fail "app role must be able to INSERT+SELECT under its tenant GUC (DML grant)"

# Fail-closed: a fresh app session with NO GUC set sees zero rows (current_group_id() → NULL → deny).
[ "$(q "$APP_TARGET" "SELECT count(*) FROM members")" = "0" ] || fail "unset tenant must see zero rows (fail-closed RLS)"

echo "✓ provision-neon meta-test passed (role-locked · 8 tables · single-owner · least-privilege+negative · public-execute · DML · RLS isolates · fail-closed · conn-string · rejects-unsafe-password)."
