#!/usr/bin/env bash
# Boundless migration live-apply test (spec 001 T06). The live half of the migration test
# strategy: applies 0001→0008 against a REAL Postgres, asserts the schema conventions hold in the
# catalog (PII columns are bytea; RLS is enabled+forced on every table), runs an RLS isolation
# smoke (a non-superuser sees only its own group's rows), then reverts 0008→0001 and asserts a
# clean teardown. The dependency-free static convention test lives in server/tests/migrations.rs.
#
# Self-skipping: if psql is absent or no database is configured, it prints a notice and exits 0,
# so it is safe to call from any hook or CI step. CI provides a postgres:18 service and sets
# DATABASE_URL (see .github/workflows/ci.yml → server-migrations).
#
# Configure via standard libpq env (PGHOST/PGPORT/PGUSER/PGPASSWORD/PGDATABASE) or DATABASE_URL.
set -euo pipefail
cd "$(dirname "$0")/.."

MIG=server/migrations

if ! command -v psql >/dev/null 2>&1; then
    echo "ℹ psql not found — skipping live migration apply test (server/tests/migrations.rs covers conventions)."
    exit 0
fi
if [ -z "${DATABASE_URL:-}" ] && [ -z "${PGHOST:-}" ] && [ -z "${PGDATABASE:-}" ]; then
    echo "ℹ no database configured (set DATABASE_URL or PG*) — skipping live migration apply test."
    exit 0
fi

# ON_ERROR_STOP makes any SQL error fail the script; --single-transaction makes each file atomic
# WITHOUT requiring in-file BEGIN/COMMIT (keeping the files sqlx-runner compatible).
PSQL=(psql -v ON_ERROR_STOP=1 --no-psqlrc -q)
[ -n "${DATABASE_URL:-}" ] && PSQL+=("$DATABASE_URL")

echo "→ applying up migrations 0001→0008…"
for f in $(printf '%s\n' "$MIG"/[0-9][0-9][0-9][0-9]_*.up.sql | sort); do
    echo "    $f"
    "${PSQL[@]}" --single-transaction -f "$f"
done

echo "→ asserting PII/secret columns are bytea (P2/I3)…"
bad=$("${PSQL[@]}" -tA <<'SQL'
SELECT format('%I.%I is %s', table_name, column_name, data_type)
FROM information_schema.columns
WHERE table_schema = 'public'
  AND (column_name ~ '(phone|token|address)' OR column_name ~ '_(hash|encrypted)$')
  AND data_type <> 'bytea';
SQL
)
if [ -n "$bad" ]; then echo "❌ non-bytea PII/secret column(s):"; echo "$bad"; exit 1; fi

echo "→ asserting RLS is enabled AND forced on every table…"
norls=$("${PSQL[@]}" -tA <<'SQL'
SELECT relname FROM pg_class
WHERE relkind = 'r' AND relnamespace = 'public'::regnamespace
  AND (NOT relrowsecurity OR NOT relforcerowsecurity);
SQL
)
if [ -n "$norls" ]; then echo "❌ table(s) without ENABLE+FORCE RLS:"; echo "$norls"; exit 1; fi

count=$("${PSQL[@]}" -tA <<'SQL'
SELECT count(*) FROM pg_class WHERE relkind = 'r' AND relnamespace = 'public'::regnamespace;
SQL
)
if [ "$count" != "8" ]; then echo "❌ expected 8 tables after apply, found $count"; exit 1; fi

echo "→ RLS isolation smoke (non-superuser): own-group visible, other-group denied, unset denied, cross-group write rejected…"
# Seed two groups as the (superuser) owner — superusers bypass RLS, so this needs no GUC. The
# smoke role below is a plain (non-superuser, non-BYPASSRLS) role, so the policies DO apply to it.
# It gets SELECT + INSERT so we can exercise both the USING (read) and WITH CHECK (write) legs.
"${PSQL[@]}" <<'SQL'
INSERT INTO groups (id, name) VALUES
  ('11111111-1111-1111-1111-111111111111', 'Group A'),
  ('22222222-2222-2222-2222-222222222222', 'Group B');
-- Reset the smoke role idempotently: DROP OWNED first (a role holding GRANTs cannot be dropped).
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'boundless_rls_smoke') THEN
    EXECUTE 'DROP OWNED BY boundless_rls_smoke';
    EXECUTE 'DROP ROLE boundless_rls_smoke';
  END IF;
END
$$;
CREATE ROLE boundless_rls_smoke NOLOGIN;
GRANT USAGE ON SCHEMA public TO boundless_rls_smoke;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO boundless_rls_smoke;
GRANT INSERT ON members TO boundless_rls_smoke;
SQL

# Drop the smoke role on ANY exit (incl. a failed assertion below) so nothing durable is left.
cleanup_smoke_role() {
    "${PSQL[@]}" -c "DROP OWNED BY boundless_rls_smoke; DROP ROLE boundless_rls_smoke;" >/dev/null 2>&1 || true
}
trap cleanup_smoke_role EXIT

# (1) USING: with the GUC set to Group A, the role sees A's row and NOT B's.
seen=$("${PSQL[@]}" -tA <<'SQL'
SET ROLE boundless_rls_smoke;
SET app.current_group_id = '11111111-1111-1111-1111-111111111111';
SELECT (SELECT count(*) FROM groups)::text || '/' ||
       (SELECT count(*) FROM groups WHERE id = '22222222-2222-2222-2222-222222222222')::text;
SQL
)
if [ "$seen" != "1/0" ]; then
    echo "❌ RLS isolation failed (own-group visible / other-group leaked = $seen, expected 1/0)"
    exit 1
fi

# (2) Deny-by-default: with the GUC UNSET, current_setting(...,true) is NULL → the role sees zero
# rows. This is the security-critical half — a connection that forgets to set the tenant fails
# CLOSED (sees nothing), never open (the whole point of the `, true` missing_ok flag).
denied=$("${PSQL[@]}" -tA <<'SQL'
SET ROLE boundless_rls_smoke;
RESET app.current_group_id;
SELECT count(*) FROM groups;
SQL
)
if [ "$denied" != "0" ]; then
    echo "❌ RLS not fail-closed: GUC unset returned $denied row(s), expected 0"
    exit 1
fi

# (3) WITH CHECK: under Group A's GUC, inserting a row tagged Group B must be REJECTED (the write
# leg of the policy). We expect this statement to error; a success is the failure.
set +e
"${PSQL[@]}" <<'SQL' >/dev/null 2>&1
SET ROLE boundless_rls_smoke;
SET app.current_group_id = '11111111-1111-1111-1111-111111111111';
INSERT INTO members (id, group_id)
VALUES ('33333333-3333-3333-3333-333333333333', '22222222-2222-2222-2222-222222222222');
SQL
check_rc=$?
set -e
if [ "$check_rc" = "0" ]; then
    echo "❌ RLS WITH CHECK failed: a cross-group INSERT under tenant A was allowed"
    exit 1
fi

cleanup_smoke_role
trap - EXIT

echo "→ reverting down migrations 0008→0001…"
for f in $(printf '%s\n' "$MIG"/[0-9][0-9][0-9][0-9]_*.down.sql | sort -r); do
    echo "    $f"
    "${PSQL[@]}" --single-transaction -f "$f"
done

echo "→ asserting clean teardown (no tables, no enum types)…"
left=$("${PSQL[@]}" -tA <<'SQL'
SELECT count(*) FROM pg_class WHERE relkind = 'r' AND relnamespace = 'public'::regnamespace;
SQL
)
if [ "$left" != "0" ]; then echo "❌ $left table(s) survived teardown"; exit 1; fi
types=$("${PSQL[@]}" -tA <<'SQL'
SELECT string_agg(typname, ', ') FROM pg_type
WHERE typnamespace = 'public'::regnamespace AND typtype = 'e';
SQL
)
if [ -n "$types" ]; then echo "❌ enum type(s) survived teardown: $types"; exit 1; fi

echo "✓ migrations: applied 0001→0008; PII columns bytea; RLS forced + isolating on 8 tables; reverted clean."
