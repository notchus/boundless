-- 0004_recovery_codes (up) — the Driver-held self-serve re-bind secret (glossary; ADR-0016 D3;
-- AC19). No expiry (driver-held), single-use, a fresh one issued on use (superseded_at lineage).
-- Riders never use these — recovery for a Rider is Admin re-issue only — and that Driver-only
-- gate is enforced in core::auth (evaluate_recovery_code) + the endpoint (T07), not as a DB
-- constraint (a CHECK cannot read members.roles portably). code_hash is the at-rest HMAC → bytea.

CREATE TABLE recovery_codes (
    id            uuid        PRIMARY KEY,
    group_id      uuid        NOT NULL,
    member_id     uuid        NOT NULL,
    code_hash     bytea       NOT NULL,
    consumed_at   timestamptz,                      -- single-use
    superseded_at timestamptz,                      -- rotation lineage (a fresh code replaces this on use)
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    created_by    uuid,
    FOREIGN KEY (member_id, group_id) REFERENCES members (id, group_id) ON DELETE CASCADE
);

CREATE INDEX recovery_codes_member_idx ON recovery_codes (member_id);
CREATE UNIQUE INDEX recovery_codes_one_live_per_member
    ON recovery_codes (member_id)
    WHERE consumed_at IS NULL AND superseded_at IS NULL;

CREATE TRIGGER recovery_codes_set_updated_at
    BEFORE UPDATE ON recovery_codes
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

ALTER TABLE recovery_codes ENABLE ROW LEVEL SECURITY;
ALTER TABLE recovery_codes FORCE ROW LEVEL SECURITY;
CREATE POLICY recovery_codes_group_isolation ON recovery_codes
    USING (group_id = current_group_id())
    WITH CHECK (group_id = current_group_id());
