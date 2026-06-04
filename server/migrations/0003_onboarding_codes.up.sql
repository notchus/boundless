-- 0003_onboarding_codes (up) — the Admin-issued, single-use first-launch secret (glossary;
-- ADR-0016 D1; AC17). Server-side TTL, attempt rate-limit, and regenerate-invalidates-prior.
-- code_hash is the at-rest HMAC-SHA256 (I3) → bytea. The lifecycle decision lives in core::auth
-- (T04) and is enforced against server time by the endpoint (T07); this is the persistence.

CREATE TABLE onboarding_codes (
    id            uuid        PRIMARY KEY,
    group_id      uuid        NOT NULL,
    member_id     uuid        NOT NULL,
    code_hash     bytea       NOT NULL,
    expires_at    timestamptz NOT NULL,             -- server-side TTL (issuance default now() + 72h)
    attempts      integer     NOT NULL DEFAULT 0,
    max_attempts  integer     NOT NULL DEFAULT 5,   -- rate-limit (5 / 15 min window enforced in T07)
    consumed_at   timestamptz,                      -- single-use: set on successful device bind
    superseded_at timestamptz,                      -- set when a regenerated code replaces this one
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    created_by    uuid,
    FOREIGN KEY (member_id, group_id) REFERENCES members (id, group_id) ON DELETE CASCADE
);

CREATE INDEX onboarding_codes_member_idx ON onboarding_codes (member_id);
-- AC17 made structural: at most one live (unconsumed, unsuperseded) code per member. T07 must
-- supersede the prior code in the same transaction as inserting a regenerated one.
CREATE UNIQUE INDEX onboarding_codes_one_live_per_member
    ON onboarding_codes (member_id)
    WHERE consumed_at IS NULL AND superseded_at IS NULL;

CREATE TRIGGER onboarding_codes_set_updated_at
    BEFORE UPDATE ON onboarding_codes
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

ALTER TABLE onboarding_codes ENABLE ROW LEVEL SECURITY;
ALTER TABLE onboarding_codes FORCE ROW LEVEL SECURITY;
CREATE POLICY onboarding_codes_group_isolation ON onboarding_codes
    USING (group_id = current_group_id())
    WITH CHECK (group_id = current_group_id());
