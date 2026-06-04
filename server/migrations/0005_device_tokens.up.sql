-- 0005_device_tokens (up) — push tokens scoped per (member, platform, app version), the exact I4
-- binding tuple (privacy-invariants I4; AC4). The DeviceToken is PII (P2) → token_encrypted is
-- bytea and never logged. invalidated_at marks a token killed by re-onboarding / revoke-logout /
-- deletion (the invalidation triggers modelled in core::auth T05).

CREATE TYPE device_platform AS ENUM
    ('ios', 'ipados', 'watchos', 'macos', 'android', 'wearos', 'web');  -- core::domain::Platform wire names

CREATE TABLE device_tokens (
    group_id        uuid            NOT NULL,
    member_id       uuid            NOT NULL,
    platform        device_platform NOT NULL,
    app_version     text            NOT NULL,
    token_encrypted bytea           NOT NULL,
    invalidated_at  timestamptz,
    created_at      timestamptz     NOT NULL DEFAULT now(),
    updated_at      timestamptz     NOT NULL DEFAULT now(),
    created_by      uuid,
    PRIMARY KEY (member_id, platform, app_version),
    FOREIGN KEY (member_id, group_id) REFERENCES members (id, group_id) ON DELETE CASCADE
);

CREATE TRIGGER device_tokens_set_updated_at
    BEFORE UPDATE ON device_tokens
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

ALTER TABLE device_tokens ENABLE ROW LEVEL SECURITY;
ALTER TABLE device_tokens FORCE ROW LEVEL SECURITY;
CREATE POLICY device_tokens_group_isolation ON device_tokens
    USING (group_id = current_group_id())
    WITH CHECK (group_id = current_group_id());
