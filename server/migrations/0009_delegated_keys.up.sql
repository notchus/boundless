-- 0009_delegated_keys (up) — the per-Group field-encryption key (the DEK), stored ONLY KEK-wrapped
-- (ADR-0025; spec 008). `wrapped_key` is `nonce ‖ ciphertext` from secretbox(group_key, KEK); the
-- plaintext key never touches durable storage or logs (P2/I1). One row per Group (single-tenant
-- install). `kek_version` makes a KEK re-wrap rotation traceable (docs/runbooks/key-management.md).

CREATE TABLE delegated_keys (
    group_id    uuid        PRIMARY KEY REFERENCES groups (id) ON DELETE CASCADE,
    wrapped_key bytea       NOT NULL,             -- Group secretbox key, KEK-wrapped; NEVER plaintext
    kek_version integer     NOT NULL DEFAULT 1,   -- bumped on a KEK re-wrap (rotation traceability)
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    created_by  uuid
);

CREATE TRIGGER delegated_keys_set_updated_at
    BEFORE UPDATE ON delegated_keys
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

ALTER TABLE delegated_keys ENABLE ROW LEVEL SECURITY;
ALTER TABLE delegated_keys FORCE ROW LEVEL SECURITY;
CREATE POLICY delegated_keys_group_isolation ON delegated_keys
    USING (group_id = current_group_id())
    WITH CHECK (group_id = current_group_id());
