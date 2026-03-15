-- Up
ALTER TABLE mods ADD COLUMN disabled_reason TEXT;

-- We don't need a Down migration for SQLite in this setup generally, 
-- but if we want to be explicit:
-- ALTER TABLE mods DROP COLUMN disabled_reason;
