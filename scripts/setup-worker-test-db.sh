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
# A SECOND Group (B) + a Group-B member, seeded so the spec-008 **T11** cross-tenant test
# (server/test/cross-tenant.spec.ts) can prove the Worker — scoped to GROUP_ID = the Group-A
# SEED_GROUP_ID above — cannot list/read/edit them (AC16 / sec-audit F5). Group B deliberately gets NO
# `delegated_keys`/KEK: Worker-A never touches Group B's crypto — RLS hides the rows before any decrypt,
# which is the whole point. Fixed ids, shared verbatim with the test (and mirroring the GB/MB convention
# in scripts/test-provision-neon.sh). The seed runs as the superuser below (bypasses RLS).
XTENANT_GROUP_ID="20000000-0000-0000-0000-000000000000"
XTENANT_MEMBER_ID="2b000000-0000-0000-0000-000000000000"

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

# Seed a SECOND tenant (Group B) + one Group-B member, as the superuser (bypasses RLS for the seed), so
# the T11 cross-tenant test can prove RLS isolation through the live Worker (AC16). No crypto needed —
# Group B has no delegated_keys and the member's PII columns stay NULL (Worker-A never decrypts them; it
# must never even SELECT them). A distinct phone_lookup_hash keeps the (group_id, phone_lookup_hash)
# index satisfied. Idempotent (ON CONFLICT DO NOTHING), so re-runs are safe.
echo "→ seeding cross-tenant Group ${XTENANT_GROUP_ID} + member ${XTENANT_MEMBER_ID} (T11 AC16 isolation)"
$PSQL "$SU_URL" -v ON_ERROR_STOP=1 -q <<SQL
INSERT INTO groups (id, name) VALUES ('${XTENANT_GROUP_ID}', 'Cross-Tenant Group B')
  ON CONFLICT (id) DO NOTHING;
-- ONE Group-B member. The FIXED updated_at (epoch 1700000000) lets the T11 edit-isolation case pass the
-- member's REAL updated_at, so the optimistic-concurrency WHERE clause WOULD match the row if it were
-- visible — making a 409 (0 rows) a genuine "RLS hid an otherwise-matching row" signal, not merely a
-- stale-timestamp miss (a NON-vacuous isolation assertion; the set_updated_at trigger is BEFORE UPDATE
-- only, so this explicit INSERT value sticks). The distinct phone_lookup_hash satisfies the
-- (group_id, phone_lookup_hash) index — bump it if you ever add a SECOND Group-B member.
INSERT INTO members (id, group_id, roles, phone_lookup_hash, updated_at)
  VALUES ('${XTENANT_MEMBER_ID}', '${XTENANT_GROUP_ID}', '{rider}'::member_role[], '\x99',
          to_timestamp(1700000000))
  ON CONFLICT (id) DO NOTHING;
SQL

echo "✓ worker test DB ready (role=${APP_ROLE}, 11 migrations applied, Group A bootstrapped + Group B cross-tenant seeded)"
