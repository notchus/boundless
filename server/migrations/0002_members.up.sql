-- 0002_members (up) — (Member, Group) pairs; the rows the auth flows read (AC3/AC4). Issuance
-- (spec 008) writes the PII; this migration defines the columns auth reads. Both phone columns
-- are PII → bytea, never text (I3, P2): phone_lookup_hash is the keyed HMAC-SHA256 used for the
-- constant-time sign-in lookup; phone_encrypted is the display-only ciphertext an Admin read
-- decrypts. Both are nullable because an Admin authenticates via WebAuthn and may have no phone.

CREATE TYPE member_role AS ENUM ('rider', 'driver', 'admin');  -- mirrors core::domain::Role wire form

CREATE TABLE members (
    id                uuid          PRIMARY KEY,
    group_id          uuid          NOT NULL REFERENCES groups (id) ON DELETE CASCADE,
    roles             member_role[] NOT NULL DEFAULT '{}',
    phone_lookup_hash bytea,
    phone_encrypted   bytea,
    created_at        timestamptz   NOT NULL DEFAULT now(),
    updated_at        timestamptz   NOT NULL DEFAULT now(),
    created_by        uuid,
    -- Composite-FK target: child tables pin (member_id, group_id) so the denormalized group_id
    -- they carry for RLS can never drift from the member's actual group.
    CONSTRAINT members_id_group_key UNIQUE (id, group_id)
);

-- One member per phone within a Group (the per-instance HMAC is deterministic). Also serves the
-- sign-in lookup, which runs under RLS scoped by group_id. Multiple NULL phones (Admins) allowed.
CREATE UNIQUE INDEX members_group_phone_lookup_key ON members (group_id, phone_lookup_hash);

CREATE TRIGGER members_set_updated_at
    BEFORE UPDATE ON members
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

ALTER TABLE members ENABLE ROW LEVEL SECURITY;
ALTER TABLE members FORCE ROW LEVEL SECURITY;
CREATE POLICY members_group_isolation ON members
    USING (group_id = current_group_id())
    WITH CHECK (group_id = current_group_id());
