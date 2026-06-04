-- 0007_admin_webauthn_credentials (up) — WebAuthn credentials for Admins (members holding the
-- admin role), registered + asserted on the SvelteKit edge (ADR-0017; AC20). An Admin may hold
-- MORE THAN ONE credential (passkey + hardware backup) → no unique on admin_id, just an index.
-- Lost-credential recovery is a Developer re-invite that revokes the prior credential(s) via
-- revoked_at (ADR-0015/0016 D4). credential_id and public_key are opaque WebAuthn material → bytea.

CREATE TABLE admin_webauthn_credentials (
    id            uuid        PRIMARY KEY,
    group_id      uuid        NOT NULL,
    admin_id      uuid        NOT NULL,
    credential_id bytea       NOT NULL,
    public_key    bytea       NOT NULL,
    sign_count    bigint      NOT NULL DEFAULT 0,
    aaguid        bytea,
    transports    text[],
    revoked_at    timestamptz,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    created_by    uuid,
    CONSTRAINT admin_webauthn_credential_id_key UNIQUE (credential_id),
    FOREIGN KEY (admin_id, group_id) REFERENCES members (id, group_id) ON DELETE CASCADE
);

CREATE INDEX admin_webauthn_admin_idx ON admin_webauthn_credentials (admin_id);  -- multiple per admin (AC20)

CREATE TRIGGER admin_webauthn_credentials_set_updated_at
    BEFORE UPDATE ON admin_webauthn_credentials
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

ALTER TABLE admin_webauthn_credentials ENABLE ROW LEVEL SECURITY;
ALTER TABLE admin_webauthn_credentials FORCE ROW LEVEL SECURITY;
CREATE POLICY admin_webauthn_credentials_group_isolation ON admin_webauthn_credentials
    USING (group_id = current_group_id())
    WITH CHECK (group_id = current_group_id());
