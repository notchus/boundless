-- 0011_audit_log (up) — the I5 admin-PII-read audit trail: every server response that returns
-- member PII to an Admin emits a row here (timestamp, admin id, member id, the field NAMES returned,
-- request id). The audit INSERT is committed in the SAME transaction as the PII read (T07), so a
-- read can never be served without its audit row.
--
-- CONVENTION DIVERGENCE (deliberate — noted here so the static convention test's carve-out is
-- justified): this table is APPEND-ONLY. It has NO `updated_at` column and NO `set_updated_at`
-- trigger (audit rows are immutable), and NO `created_by` (the actor is `admin_id` itself). `fields`
-- stores field NAMES, never values (AC9), so the table holds NO PII and has NO `_encrypted` column —
-- reading the audit log is therefore not itself a recursive PII read. `id` is DB-minted
-- (`gen_random_uuid()`, core since PG13 — no pgcrypto) since audit rows are written server-side: this
-- is the ONE deliberate exception to the schema's app-minted-uuid-PK convention (the audit id is
-- opaque, never client-supplied or wire-round-tripped, so a DB default is simpler and safe — it does
-- NOT license DB-minted PKs elsewhere). Group-scoped ENABLE+FORCE RLS like every PII table, so
-- AC16's cross-tenant proof covers it too.
--
-- PROVISIONAL `ON DELETE CASCADE` (below): pending the I12 deletion design. I12 keeps audit logs
-- (legal requirement) with PII redacted — but CASCADE would *delete* a forgotten member's audit
-- rows. When `core::deletion::forget_member` is specced it must reconcile this (e.g. SET NULL +
-- an `Anonymous_NNNN` ref, or sever the FK) so the trail is retained, not dropped. Tracked in
-- DEFERRED.md → spec 008.

CREATE TABLE audit_log (
    id         uuid        PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id   uuid        NOT NULL,
    admin_id   uuid        NOT NULL,          -- the acting Admin (I5)
    member_id  uuid        NOT NULL,          -- whose PII was read
    fields     text[]      NOT NULL,          -- field NAMES, never values (AC9), e.g. {'address','phone'}
    request_id text        NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    -- (member_id, group_id) so the denormalized group_id carried for RLS cannot drift from the member.
    FOREIGN KEY (member_id, group_id) REFERENCES members (id, group_id) ON DELETE CASCADE
);

CREATE INDEX audit_log_member_idx ON audit_log (member_id);

ALTER TABLE audit_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_log FORCE ROW LEVEL SECURITY;
CREATE POLICY audit_log_group_isolation ON audit_log
    USING (group_id = current_group_id())
    WITH CHECK (group_id = current_group_id());
