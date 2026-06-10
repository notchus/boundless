-- 0010_member_pii (up) — the at-rest PII columns issuance (spec 008) writes on `members`: the
-- member's name and home address, each encrypted with the per-Group secretbox key as `nonce ‖
-- ciphertext` (I1/AC2/AC3, ADR-0025). Both nullable: existing rows backfill NULL, and an Admin
-- (WebAuthn, no phone) need not carry an address. Decided to live on `members` (not a separate
-- table): same RLS surface, phone PII already here, one-row I12 sweep (ADR-0006 precedent).
-- No new table/trigger/policy — `members` already ENABLE+FORCE RLS (0002), which covers these columns.

ALTER TABLE members
    ADD COLUMN name_encrypted    bytea,
    ADD COLUMN address_encrypted bytea;
