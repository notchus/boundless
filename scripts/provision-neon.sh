#!/usr/bin/env bash
# Provision a REAL Boundless Postgres (Neon) for the deployable Worker — spec 001 T07-shell-B.
#
# This is the ONE non-`wrangler` step of a deploy (docs/runbooks/deploy-worker.md). Run it ONCE,
# as the database OWNER (Neon's `neondb_owner`), against an existing database. It is **non-destructive
# and idempotent** — unlike scripts/setup-worker-test-db.sh, which is a DESTRUCTIVE test-DB reset
# (`DROP SCHEMA public CASCADE`) and must never touch a real DB.
#
# What it does (all as the owner):
#   * creates/repairs the runtime app role `boundless_app` with the safe DEFAULTS (NOSUPERUSER
#     NOBYPASSRLS) and then VERIFIES it is unprivileged — so the W2 `ensure_least_privilege` guard
#     ACCEPTS it and RLS actually applies. It deliberately does NOT `ALTER … NOSUPERUSER/NOBYPASSRLS`:
#     only a true superuser may change those, and the owner that runs this (Neon's `neondb_owner`) is a
#     CREATEROLE role, NOT a superuser, so naming them is rejected. (Neon's `neondb_owner` itself has
#     BYPASSRLS and is correctly REJECTED by the Worker's boot guard — DEFERRED.md → T07-shell-B,
#     sec-audit W2/R3 — which is why this script mints a separate, locked-down role.)
#   * applies the schema migrations IDEMPOTENTLY — each `server/migrations/NNNN_*.up.sql` is applied
#     only if its MARKER object is absent, so a fresh DB gets all of them AND an older DB (e.g. one
#     provisioned before 0009..0011 existed) picks up exactly the new ones. It never DROPs and never
#     re-runs a CREATE on an existing object; an unmapped migration (no marker) is a hard error;
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

# All human-facing output goes to stderr; STDOUT is reserved for the one connection-string line.
log() { echo "$@" >&2; }
# A redacted scheme://host/db for logging — NEVER the userinfo (P2: the owner password must not reach
# logs/stderr). The greedy `.*@` strips ALL userinfo (robust even if the password contains '@'); then drop
# any `?query`. (`%%@*` would have kept `scheme://user:PW` — a password leak.)
safe_url="$(printf '%s' "$OWNER_URL" | sed -E 's#^(postgres(ql)?://).*@#\1#; s#\?.*$##')"
log "→ provisioning ${safe_url} as the database owner (app role=${APP_ROLE})"

# --- 0. pre-flight: reject a pooled host, then connect, then require an owner that can create roles --
# Neon's pooled endpoint (-pooler) blocks the SET/prepared statements migrations need AND must never be the
# Hyperdrive origin (Hyperdrive pools itself) — a pooled host here would silently ride into `wrangler
# hyperdrive create` (step 1). FATAL, checked first (string-only, no connection); matched on the redacted
# host so a '-pooler' in the password can't trip it. Set ALLOW_POOLER=1 to override a non-Neon pooler.
case "$safe_url" in
  *-pooler.*)
    if [ "${ALLOW_POOLER:-}" != "1" ]; then
      log "✗ this is a POOLED endpoint ('-pooler' in the host). Use the DIRECT (unpooled) endpoint —"
      log "  Neon dashboard → Connection Details → turn Connection pooling OFF (host without '-pooler')."
      log "  DDL needs it, and Hyperdrive (step 1) must point at the direct host. Set ALLOW_POOLER=1 to override."
      exit 1
    fi
    log "⚠ ALLOW_POOLER=1 — provisioning via a POOLED endpoint; migrations may fail and the printed host is"
    log "  NOT suitable for 'wrangler hyperdrive create'." ;;
esac
# Connect (fail clearly instead of surfacing a raw psql error mid-run).
if ! $PSQL "$OWNER_URL" -tAc 'SELECT 1' >/dev/null 2>&1; then
  log "✗ cannot connect with the given owner URL."
  log "  Check host/role/password, and use the DIRECT (unpooled) endpoint (host WITHOUT '-pooler')."
  exit 1
fi
# Accept a true superuser OR a CREATEROLE owner (a superuser can create roles even with rolcreaterole=f).
if [ "$($PSQL "$OWNER_URL" -tAc 'SELECT (rolsuper OR rolcreaterole) FROM pg_roles WHERE rolname = current_user' 2>/dev/null | tr -d '[:space:]')" != "t" ]; then
  log "✗ the connection role can't create roles (needs the database OWNER/CREATEROLE, or superuser)."
  log "  On Neon connect as 'neondb_owner' (NOT 'authenticator', the limited pooler/Authorize role):"
  log "  Neon dashboard → Connection Details → role = neondb_owner."
  exit 1
fi

# --- 1. app role (UNCONDITIONAL — re-asserted every run) ---------------------------------------
# Create with the safe DEFAULTS (NOSUPERUSER NOBYPASSRLS are the defaults for any CREATEROLE-created
# role) and re-assert only LOGIN + password. We deliberately do NOT name SUPERUSER/BYPASSRLS in ALTER:
# changing those requires the EXECUTOR to be a true superuser, and the owner that runs this (Neon's
# `neondb_owner`) is a CREATEROLE role, not a superuser — naming them is rejected ("only roles with the
# SUPERUSER attribute may change the SUPERUSER attribute"). The verify step below ENFORCES least
# privilege (a drifted privileged role can only be fixed by a superuser, so we detect + refuse here).
$PSQL "$OWNER_URL" -v ON_ERROR_STOP=1 -q <<SQL
DO \$\$ BEGIN
  CREATE ROLE ${APP_ROLE} LOGIN PASSWORD '${APP_PW}';
EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL;
END \$\$;
ALTER ROLE ${APP_ROLE} LOGIN PASSWORD '${APP_PW}';
SQL

# Verify it landed unprivileged. SUPERUSER/BYPASSRLS bypass every RLS policy; REPLICATION can stream the
# whole WAL (all tenants' PII, bypassing RLS); CREATEROLE/CREATEDB are role/db escalation. A re-run or a
# pre-existing role preserves whatever attributes it already had (the CREATE is swallowed, the ALTER touches
# only LOGIN+password), so check them all and fail loudly — only a superuser could reset a drifted role.
attr() { $PSQL "$OWNER_URL" -tAc "SELECT $1 FROM pg_roles WHERE rolname='${APP_ROLE}'" | tr -d '[:space:]'; }
[ "$(attr rolsuper)"       = "f" ] || { log "✗ ${APP_ROLE} is SUPERUSER — a superuser must run:  ALTER ROLE ${APP_ROLE} NOSUPERUSER;"; exit 1; }
[ "$(attr rolbypassrls)"   = "f" ] || { log "✗ ${APP_ROLE} has BYPASSRLS — a superuser must run:  ALTER ROLE ${APP_ROLE} NOBYPASSRLS;"; exit 1; }
[ "$(attr rolreplication)" = "f" ] || { log "✗ ${APP_ROLE} has REPLICATION (can stream all tenants' data, bypassing RLS) — a superuser must run:  ALTER ROLE ${APP_ROLE} NOREPLICATION;"; exit 1; }
[ "$(attr rolcreaterole)"  = "f" ] || { log "✗ ${APP_ROLE} has CREATEROLE — run:  ALTER ROLE ${APP_ROLE} NOCREATEROLE;"; exit 1; }
[ "$(attr rolcreatedb)"    = "f" ] || { log "✗ ${APP_ROLE} has CREATEDB — run:  ALTER ROLE ${APP_ROLE} NOCREATEDB;"; exit 1; }
[ "$(attr rolcanlogin)"    = "t" ] || { log "✗ ${APP_ROLE} cannot LOGIN —  run:  ALTER ROLE ${APP_ROLE} LOGIN;"; exit 1; }
log "✓ app role ${APP_ROLE} (LOGIN · verified NOSUPERUSER · NOBYPASSRLS · NOREPLICATION · NOCREATEROLE · NOCREATEDB)"

# --- 2. migrations (IDEMPOTENT — never destructive) --------------------------------------------
# Apply each up-migration whose MARKER object is absent; skip those already applied. This makes a fresh
# DB get all of them AND an already-partially-migrated DB (e.g. an older deploy on 0001..0008 from before
# 0009..0011 existed) pick up exactly the new ones — repeatable for every future migration. We never DROP
# and never re-run a CREATE on an existing object: each file is wrapped --single-transaction (atomic ⇒ one
# marker reliably means "fully applied"), and the marker probe gates the apply.
#
# marker_sql <basename> → a SQL predicate, TRUE iff this migration is already applied. Tables probe
# `to_regclass`; 0010 ALTERs `members` (creates no table) so it probes the column. An UNMAPPED migration
# (a future NNNN_*.up.sql with no entry) is a HARD ERROR — fail closed so a new migration can never be
# silently skipped or silently re-applied (it MUST be added here with its marker).
marker_sql() {
  case "$1" in
    0001_*) echo "to_regclass('public.groups') IS NOT NULL" ;;
    0002_*) echo "to_regclass('public.members') IS NOT NULL" ;;
    0003_*) echo "to_regclass('public.onboarding_codes') IS NOT NULL" ;;
    0004_*) echo "to_regclass('public.recovery_codes') IS NOT NULL" ;;
    0005_*) echo "to_regclass('public.device_tokens') IS NOT NULL" ;;
    0006_*) echo "to_regclass('public.sessions') IS NOT NULL" ;;
    0007_*) echo "to_regclass('public.admin_webauthn_credentials') IS NOT NULL" ;;
    0008_*) echo "to_regclass('public.admin_invitations') IS NOT NULL" ;;
    0009_*) echo "to_regclass('public.delegated_keys') IS NOT NULL" ;;
    0010_*) echo "EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema='public' AND table_name='members' AND column_name='name_encrypted')" ;;
    0011_*) echo "to_regclass('public.audit_log') IS NOT NULL" ;;
    *) return 1 ;;
  esac
}

shopt -s nullglob
mig_files=("$MIG_DIR"/*.up.sql)
[ "${#mig_files[@]}" -ge 1 ] || { log "✗ no migrations found in ${MIG_DIR}"; exit 1; }
applied=0; skipped=0
for f in "${mig_files[@]}"; do
  name="$(basename "$f" .up.sql)"
  if ! marker="$(marker_sql "$name")"; then
    log "✗ migration ${name} has no marker mapping in provision-neon.sh — refusing to guess."
    log "  Add a marker_sql case for it (the object it creates), then re-run."
    exit 1
  fi
  has="$($PSQL "$OWNER_URL" -tAc "SELECT ${marker}")"
  has="$(printf '%s' "$has" | tr -d '[:space:]')"
  case "$has" in
    t) skipped=$((skipped + 1)) ;;
    # Fed on stdin (not -f) so `docker exec -i` works; --single-transaction wraps each file.
    f) $PSQL "$OWNER_URL" -v ON_ERROR_STOP=1 -q --single-transaction < "$f"; log "  ✓ applied ${name}"; applied=$((applied + 1)) ;;
    *) log "✗ could not probe the marker for ${name} (got: '${has}')"; exit 1 ;;
  esac
done
log "✓ migrations: ${applied} applied, ${skipped} already present ($((applied + skipped)) total)"

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
