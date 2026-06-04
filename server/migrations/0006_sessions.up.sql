-- 0006_sessions (up) — indefinite member sessions with silent refresh-token rotation and
-- replay/lineage detection (ADR-0016 D2; AC18; backs core::auth T05). Each row is ONE refresh
-- credential within a family (family_id = core::domain::SessionFamilyId). The family's single
-- live credential is the row with rotated_at IS NULL AND revoked_at IS NULL; rotating it sets
-- rotated_at and inserts a child (parent_id). This is exactly T05's RefreshPresentation:
--   * presented hash matches the live row            → Current   → rotate
--   * matches a rotated row on a still-live family    → Superseded → replay → revoke the family
--   * matches nothing live                            → Unknown   → reject
-- A session has no expiry column: it lives until revoked_at is set (admin event or replay).
-- refresh_token_hash is the at-rest HMAC, never plaintext → bytea.

CREATE TABLE sessions (
    id                 uuid        PRIMARY KEY,                 -- this credential's id
    group_id           uuid        NOT NULL,
    member_id          uuid        NOT NULL,
    family_id          uuid        NOT NULL,                    -- revoked as a unit on replay / admin event
    refresh_token_hash bytea       NOT NULL,
    parent_id          uuid        REFERENCES sessions (id) ON DELETE SET NULL,  -- credential this rotated from
    rotated_at         timestamptz,                             -- set when superseded by its child
    revoked_at         timestamptz,                             -- family kill; otherwise indefinite
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),
    created_by         uuid,
    FOREIGN KEY (member_id, group_id) REFERENCES members (id, group_id) ON DELETE CASCADE
);

CREATE INDEX sessions_family_idx ON sessions (family_id);
CREATE UNIQUE INDEX sessions_refresh_token_hash_key ON sessions (refresh_token_hash);
-- The rotation invariant made structural: exactly one live (current) credential per family.
CREATE UNIQUE INDEX sessions_one_current_per_family
    ON sessions (family_id)
    WHERE rotated_at IS NULL AND revoked_at IS NULL;

CREATE TRIGGER sessions_set_updated_at
    BEFORE UPDATE ON sessions
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

ALTER TABLE sessions ENABLE ROW LEVEL SECURITY;
ALTER TABLE sessions FORCE ROW LEVEL SECURITY;
CREATE POLICY sessions_group_isolation ON sessions
    USING (group_id = current_group_id())
    WITH CHECK (group_id = current_group_id());
