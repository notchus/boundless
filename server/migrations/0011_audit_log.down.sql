-- 0011_audit_log (down). Drops before 0002 drops `members` (downs revert in reverse order), so the
-- (member_id, group_id) FK is gone first.
DROP TABLE IF EXISTS audit_log;
