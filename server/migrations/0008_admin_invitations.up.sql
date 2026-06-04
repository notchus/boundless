-- 0008_admin_invitations (up) — developer-minted, single-use, server-TTL registration links for
-- pending Admins (I11 as narrowed by ADR-0015; AC16). The link carries only an opaque token; the
-- row stores only its at-rest HMAC (token_hash → bytea), never the token or any PII/credential.
-- Consumed on the first successful WebAuthn registration (consumed_at). created_by is the Developer.

CREATE TABLE admin_invitations (
    id          uuid        PRIMARY KEY,
    group_id    uuid        NOT NULL,
    admin_id    uuid        NOT NULL,             -- the pending Admin (member) this invites
    token_hash  bytea       NOT NULL,
    expires_at  timestamptz NOT NULL,             -- server-side TTL (mint default now() + 72h)
    consumed_at timestamptz,                      -- single-use
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    created_by  uuid,                             -- the Developer who minted it (I11)
    CONSTRAINT admin_invitations_token_hash_key UNIQUE (token_hash),
    FOREIGN KEY (admin_id, group_id) REFERENCES members (id, group_id) ON DELETE CASCADE
);

-- At most one outstanding (unconsumed) invitation per pending admin; a re-invite supersedes by
-- consuming/expiring the prior one in the same transaction (T08).
CREATE UNIQUE INDEX admin_invitations_one_live_per_admin
    ON admin_invitations (admin_id)
    WHERE consumed_at IS NULL;

CREATE TRIGGER admin_invitations_set_updated_at
    BEFORE UPDATE ON admin_invitations
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

ALTER TABLE admin_invitations ENABLE ROW LEVEL SECURITY;
ALTER TABLE admin_invitations FORCE ROW LEVEL SECURITY;
CREATE POLICY admin_invitations_group_isolation ON admin_invitations
    USING (group_id = current_group_id())
    WITH CHECK (group_id = current_group_id());
