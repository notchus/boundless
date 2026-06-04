-- 0001_groups (down). Applied last in a reverse run, so every dependent table (and its
-- updated_at trigger) is already gone by the time the shared function is dropped.
DROP TABLE IF EXISTS groups;
DROP FUNCTION IF EXISTS current_group_id();
DROP FUNCTION IF EXISTS set_updated_at();
