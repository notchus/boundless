-- 0001_groups (up) — the FK anchor: one closed Group per Boundless install (domain glossary).
--
-- Conventions enforced from here on (docs/stack-matrix.md "Schema conventions"; plan §3):
--   * every row carries created_at / updated_at / created_by (actor for audit, I5);
--   * field-level PII / secrets are bytea, never text (none on this table);
--   * row-level security is ENABLEd and FORCEd on every table, scoped by the per-request GUC
--     app.current_group_id (one install = one Group today, but the isolation is uniform so it
--     holds if that ever changes, and so Hyperdrive can set the tenant per connection).
-- No in-file BEGIN/COMMIT: the runner (sqlx::migrate! / psql --single-transaction) wraps each
-- file in its own transaction. Filenames follow sqlx's reversible convention (NNNN_*.{up,down}.sql).

-- Shared updated_at trigger function: defined once here, attached by every table's migration.
-- plpgsql is required because a SQL function cannot assign to NEW.
CREATE FUNCTION set_updated_at() RETURNS trigger
    LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END;
$$;

-- The single fail-closed tenant resolver used by every RLS policy below. It maps BOTH an unset
-- GUC (current_setting(..., true) → NULL) AND a reset-to-empty GUC ('' — what a pooled Hyperdrive
-- connection yields after RESET) to NULL, so `group_id = current_group_id()` is NULL → the row is
-- denied. Centralizing it means there is exactly ONE place to audit that RLS denies by default,
-- and that an unset tenant yields zero rows rather than a `''::uuid` cast error. STABLE: the GUC
-- is constant within a statement.
CREATE FUNCTION current_group_id() RETURNS uuid
    LANGUAGE sql STABLE AS
$$ SELECT NULLIF(current_setting('app.current_group_id', true), '')::uuid $$;

CREATE TABLE groups (
    id         uuid        PRIMARY KEY,
    name       text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    created_by uuid                                  -- developer/actor id for audit (I5); NULL = system
);

CREATE TRIGGER groups_set_updated_at
    BEFORE UPDATE ON groups
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

ALTER TABLE groups ENABLE ROW LEVEL SECURITY;
ALTER TABLE groups FORCE ROW LEVEL SECURITY;
CREATE POLICY groups_group_isolation ON groups
    USING (id = current_group_id())
    WITH CHECK (id = current_group_id());
