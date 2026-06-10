-- 0010_member_pii (down). Runs before 0002 drops `members` (downs revert in reverse order).
ALTER TABLE members
    DROP COLUMN IF EXISTS address_encrypted,
    DROP COLUMN IF EXISTS name_encrypted;
