#!/usr/bin/env bash
# Provision a REAL Boundless Postgres (Neon) for the deployable Worker — spec 001 T07-shell-B.
#
# This is the ONE non-`wrangler` step of a deploy (docs/runbooks/deploy-worker.md). Run it ONCE,
# as the database OWNER (Neon's `neondb_owner`), against an existing database. It is **non-destructive
# and idempotent** — unlike scripts/setup-worker-test-db.sh, which is a DESTRUCTIVE test-DB reset
# (`DROP SCHEMA public CASCADE`) and must never touch a real DB.
#
# What it does (all as the owner):
#   * creates/repairs the runtime app role `boundless_app` — **LOGIN NOSUPERUSER NOBYPASSRLS** so the
#     W2 `boundless_server_store::ensure_least_privilege` guard ACCEPTS it and RLS actually applies
#     (Neon's default `neondb_owner` has BYPASSRLS and is correctly REJECTED — DEFERRED.md → T07-shell-B,
#     sec-audit W2/R3);
#   * applies the T06 migrations 0001..0008 **only if the schema is empty** (0 of the 8 tables present);
#     if all 8 are present it skips; a PARTIAL schema makes it fail loudly (it never DROPs / never guesses);
#   * grants the app role CONNECT + USAGE + table DML (the proven server/store/tests/common/mod.rs set;
#     no extension/sequence/EXECUTE grants are needed — gen_random_uuid is a PG13+ builtin, and functions
#     keep the PG15+ default PUBLIC EXECUTE). Grants run on EVERY run, even when migrations are skipped.
#   * prints the ready-to-paste **app-role connection string** to STDOUT (one line; all progress goes to
#     STDERR), i.e. the exact `--connection-string` for `wrangler hyperdrive create` (runbook step 1).
#
# Usage:
#   bash scripts/provision-neon.sh "postgresql://neondb_owner:PW@HOST/neondb?sslmode=require"
#   # → copy the single stdout line into:  wrangler hyperdrive create boundless-pg --connection-string "<that>"
#
# Env:
#   BOUNDLESS_APP_DB_PASSWORD  app-role password (default: a fresh `openssl rand -hex 24`). The printed
#                              connection string embeds it — handle it as a credential.
#   PSQL                       the psql invocation (default `psql`). For a containerized owner DB:
#                              PSQL="docker exec -i <container> psql"  (all SQL is fed on stdin, so this works).
set -euo pipefail

# --- args + config -----------------------------------------------------------------------------
OWNER_URL="${1:-}"
if [ -z "$OWNER_URL" ]; then
  echo "usage: provision-neon.sh <owner-connection-url>   (e.g. the Neon neondb_owner URL)" >&2
  exit 2
fi
PSQL="${PSQL:-psql}"
APP_ROLE="boundless_app"
# Default to a fresh strong hex password (hex → no SQL/URL quoting hazards). Override to pin it.
APP_PW="${BOUNDLESS_APP_DB_PASSWORD:-$(openssl rand -hex 24)}"
# APP_PW is interpolated into SQL (single-quoted) and into the emitted connection string via sed — where
# an `&` would expand to the matched owner userinfo and SILENTLY leak the owner's (BYPASSRLS) credential
# into the printed artifact, and `'`/`#` would break the SQL/sed. Require a URL+SQL-safe charset so an
# operator override can't do any of that; the `openssl rand -hex 24` default satisfies it.
case "$APP_PW" in
  ''|*[!A-Za-z0-9._~-]*)
    echo "✗ BOUNDLESS_APP_DB_PASSWORD must be non-empty and URL/SQL-safe (chars: A-Za-z0-9 . _ ~ -)" >&2
    exit 2 ;;
esac
MIG_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../server/migrations" && pwd)"

# The 8 tables the T06 migrations create (used to detect empty / migrated / partial).
EXPECTED_TABLES="groups members onboarding_codes recovery_codes device_tokens sessions admin_webauthn_credentials admin_invitations"
EXPECTED_COUNT=8

# All human-facing output goes to stderr; STDOUT is reserved for the one connection-string line.
log() { echo "$@" >&2; }
log "→ provisioning ${OWNER_URL%%@*}@… as the database owner (app role=${APP_ROLE})"

# --- 1. app role (UNCONDITIONAL — re-asserted every run) ---------------------------------------
# Idempotent create (swallow the duplicate-at-create races), then ALTER to re-assert the exact
# least-privilege attributes + password so a stale role can never drift privileged.
$PSQL "$OWNER_URL" -v ON_ERROR_STOP=1 -q <<SQL
DO \$\$ BEGIN
  CREATE ROLE ${APP_ROLE} LOGIN PASSWORD '${APP_PW}' NOSUPERUSER NOBYPASSRLS;
EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL;
END \$\$;
ALTER ROLE ${APP_ROLE} LOGIN PASSWORD '${APP_PW}' NOSUPERUSER NOBYPASSRLS;
SQL
log "✓ app role ${APP_ROLE} (LOGIN NOSUPERUSER NOBYPASSRLS)"

# --- 2. migrations (CONDITIONAL — never destructive) -------------------------------------------
# Count how many of the 8 tables already exist, then: 0 → apply all; 8 → skip; partial → refuse.
in_list="$(printf "'%s'," $EXPECTED_TABLES)"; in_list="${in_list%,}"
present="$($PSQL "$OWNER_URL" -tAc \
  "SELECT count(*) FROM pg_tables WHERE schemaname='public' AND tablename IN (${in_list})")"
present="$(printf '%s' "$present" | tr -d '[:space:]')"
# A successful `SELECT count(*)` always yields a number; guard so an empty/non-numeric read can't be
# treated as 0 (bash `[ "" -eq 0 ]` is true) → silently applying migrations onto an unknown schema.
case "$present" in ''|*[!0-9]*) log "✗ could not read the table count from the database"; exit 1 ;; esac

if [ "$present" -eq 0 ]; then
  log "→ empty schema — applying migrations 0001..0008"
  shopt -s nullglob
  mig_count=0
  for f in "$MIG_DIR"/*.up.sql; do
    # Fed on stdin (not -f) so `docker exec -i` works; --single-transaction wraps each file.
    $PSQL "$OWNER_URL" -v ON_ERROR_STOP=1 -q --single-transaction < "$f"
    mig_count=$((mig_count + 1))
  done
  if [ "$mig_count" -ne "$EXPECTED_COUNT" ]; then
    log "✗ expected ${EXPECTED_COUNT} up-migrations, applied ${mig_count}"
    exit 1
  fi
  log "✓ applied ${mig_count} migrations"
elif [ "$present" -eq "$EXPECTED_COUNT" ]; then
  log "✓ schema already migrated (${present}/${EXPECTED_COUNT} tables) — skipping migrations"
else
  log "✗ partial schema: ${present}/${EXPECTED_COUNT} expected tables present."
  log "  Refusing to guess (this script never DROPs). Inspect the database and reconcile by hand."
  exit 1
fi

# --- 3. grants (UNCONDITIONAL — re-asserted every run, AFTER migrate so ON ALL TABLES sees them) ---
# CONNECT (via format() since GRANT can't take current_database() directly) + USAGE + table DML. No
# DDL, no sequence grants (uuid PKs), no EXECUTE grants (functions are PUBLIC-EXECUTE by default).
$PSQL "$OWNER_URL" -v ON_ERROR_STOP=1 -q <<SQL
DO \$\$ BEGIN
  EXECUTE format('GRANT CONNECT ON DATABASE %I TO ${APP_ROLE}', current_database());
END \$\$;
GRANT USAGE ON SCHEMA public TO ${APP_ROLE};
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO ${APP_ROLE};
SQL
log "✓ grants (CONNECT + USAGE + table DML)"

# --- 4. emit the app-role connection string (the artifact) -------------------------------------
# Swap the WHOLE authority's userinfo for ${APP_ROLE}:${APP_PW} ([^/]*@ is greedy to the last `@`
# before the path — robust to `@` inside the old password and to a no-password owner URL) and DROP any
# owner query string. We emit a BARE string (no `?sslmode=…`): `wrangler hyperdrive create` takes TLS
# via its own `--sslmode require` flag and its documented `--connection-string` example carries no query
# (passing both is undocumented). The runbook step 1 supplies `--sslmode require`. (APP_PW is charset-
# validated above, so it can't introduce a sed `&`/delimiter or a stray `@` here.)
base="${OWNER_URL%%\?*}"
app_base="$(printf '%s' "$base" | sed -E "s#^(postgres(ql)?://)[^/]*@#\1${APP_ROLE}:${APP_PW}@#")"
case "$app_base" in
  *"//${APP_ROLE}:"*) : ;;  # userinfo swap succeeded
  *)
    log "✗ could not derive an app-role URL from the owner URL (no userinfo to swap?)."
    log "  Build it by hand: postgresql://${APP_ROLE}:<password>@<host>/<db>"
    exit 1
    ;;
esac
conn="${app_base}"

log ""
log "──────────────────────────────────────────────────────────────────────────"
log "App-role connection string (a CREDENTIAL — paste into the \`wrangler hyperdrive create"
log "  --connection-string \"…\" --sslmode require\` of docs/runbooks/deploy-worker.md step 1):"
log "  (printed on stdout below; \`provision-neon.sh … | tail -1\` yields just the string)"
log "──────────────────────────────────────────────────────────────────────────"
printf '%s\n' "$conn"
