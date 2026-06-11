#!/usr/bin/env bash
# Provision Postgres for the boundless-worker miniflare test (spec 001 T07-shell-B, PgAuthStore slice).
#
# Account-free: the vitest harness boots workerd via @cloudflare/vitest-pool-workers and the Worker
# connects to THIS Postgres over an emulated Hyperdrive Socket. This script makes the DB the Worker
# expects — idempotent, safe to re-run:
#   * creates the non-superuser LOGIN role the Worker connects as (NOSUPERUSER NOBYPASSRLS, so RLS
#     actually applies and the W2 `ensure_least_privilege` guard ACCEPTS it — the production-faithful
#     shape, unlike the store tests which connect as superuser + `SET ROLE`);
#   * (re)creates the `public` schema and applies the T06 migrations (0001..0008) into it;
#   * grants the app role table DML.
# It does NOT seed any member — the worker test asserts `phone_not_on_file` against an EMPTY db (the
# member-matched-over-real-PG path is already proven by server/store/tests/service_pg.rs).
#
# Mirrors server/store/tests/common/mod.rs::setup (minus seeding + the per-test schema). Local default
# targets the standard PG18 dev container on :55432; CI overrides the two URLs to its service (:5432).
set -euo pipefail

# Superuser connection (creates the role + runs DDL). Local default = the dev container on :55432.
SU_URL="${WORKER_TEST_SUPERUSER_URL:-postgres://postgres:postgres@localhost:55432/boundless_test}"
# The psql invocation. Default = a host `psql` (CI ubuntu ships it). A dev whose Postgres is only in
# the docker container (no host psql) sets, e.g.:
#   PSQL="docker exec -i boundless-postgres psql" \
#   WORKER_TEST_SUPERUSER_URL=postgres://postgres:postgres@localhost:5432/boundless_test \
#   bash scripts/setup-worker-test-db.sh
# (in-container the DB is on :5432). All SQL is fed on stdin so it works through `docker exec -i`.
PSQL="${PSQL:-psql}"
APP_ROLE="boundless_app"
# Must match the password in WORKER_TEST_PG / wrangler.toml localConnectionString (a LOCAL TEST cred,
# not a secret — like the postgres:postgres in common/mod.rs).
APP_PW="${WORKER_TEST_APP_PASSWORD:-boundless_app}"
MIG_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../server/migrations" && pwd)"
SERVER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../server" && pwd)"
# The single-install tenant the Worker is RLS-scoped to (the wrangler.toml `[vars] GROUP_ID`), and the
# test KEK the Worker unwraps the seeded Group key with. The KEK MUST match `server/vitest.config.ts`'s
# `KEK` binding (a LOCAL TEST value, not a secret — like the boundless_app password). Spec 008 T09.
SEED_GROUP_ID="${WORKER_TEST_GROUP_ID:-00000000-0000-0000-0000-000000000001}"
SEED_KEK_HEX="${WORKER_TEST_KEK_HEX:-cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd}"

echo "→ provisioning worker test DB via ${SU_URL%@*}@… (role=${APP_ROLE}, schema=public)"

# Role (idempotent: swallow the duplicate-at-create races, then ALTER to the desired shape) + a fresh
# public schema. The role attributes are re-asserted every run so a stale role can't drift privileged.
$PSQL "$SU_URL" -v ON_ERROR_STOP=1 -q <<SQL
-- Safety rail: this is a destructive (DROP SCHEMA ... CASCADE) test-DB reset. Refuse to run unless
-- the target database name looks like a test DB, so a fat-fingered SU_URL can't wipe a real database.
DO \$\$ BEGIN
  IF current_database() !~ '(test|boundless_test)' THEN
    RAISE EXCEPTION 'refusing to reset non-test database %', current_database();
  END IF;
END \$\$;
DO \$\$ BEGIN
  CREATE ROLE ${APP_ROLE} LOGIN PASSWORD '${APP_PW}' NOSUPERUSER NOBYPASSRLS;
EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL;
END \$\$;
ALTER ROLE ${APP_ROLE} LOGIN PASSWORD '${APP_PW}' NOSUPERUSER NOBYPASSRLS;
DROP SCHEMA IF EXISTS public CASCADE;
CREATE SCHEMA public;
SQL

# Apply each up-migration in its own transaction (the files carry no BEGIN/COMMIT by design — see the
# header in 0001). Default search_path resolves to public, so objects land there.
shopt -s nullglob
mig_count=0
for f in "$MIG_DIR"/*.up.sql; do
  # Fed on stdin (not -f) so it works through `docker exec -i` too; --single-transaction wraps it.
  $PSQL "$SU_URL" -v ON_ERROR_STOP=1 -q --single-transaction < "$f"
  mig_count=$((mig_count + 1))
done
# Guards a glob/path bug (server/tests/migrations.rs validates the set itself). Bump when 0012+ lands.
if [ "$mig_count" -ne 11 ]; then
  echo "✗ expected 11 up-migrations, applied ${mig_count}" >&2
  exit 1
fi

# Least privilege: USAGE + table DML only (no DDL, no sequences — the schema uses uuid PKs).
$PSQL "$SU_URL" -v ON_ERROR_STOP=1 -q <<SQL
GRANT USAGE ON SCHEMA public TO ${APP_ROLE};
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO ${APP_ROLE};
SQL

# Seed a bootstrapped Group (a `groups` row + a KEK-wrapped `delegated_keys` row) so the spec-008 T09
# admin-issuance tests can encrypt/decrypt PII (issuance fails closed without a per-Group key, AC12).
# Done in Rust (the Group-key wrap is `boundless_crypto::wrap_group_key`, P4 — not an opaque SQL blob)
# via a never-shipped example. Connects over TCP to SU_URL, so SU_URL must be host-reachable (the same
# port the Worker's Hyperdrive binding uses — local :55432 / CI :5432).
echo "→ seeding bootstrapped Group ${SEED_GROUP_ID} (KEK-wrapped delegated key)"
( cd "$SERVER_DIR" && \
  WORKER_TEST_SUPERUSER_URL="$SU_URL" \
  WORKER_TEST_GROUP_ID="$SEED_GROUP_ID" \
  WORKER_TEST_KEK_HEX="$SEED_KEK_HEX" \
  cargo run -q -p boundless-server-store --example seed_worker_test_pg )

echo "✓ worker test DB ready (role=${APP_ROLE}, 11 migrations applied, 1 bootstrapped Group seeded)"
