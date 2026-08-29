ALTER TABLE import_jobs ADD COLUMN source_group_id TEXT;
ALTER TABLE import_jobs ADD COLUMN processed_source_path TEXT;
ALTER TABLE import_jobs ADD COLUMN source_processed_at TEXT;
ALTER TABLE import_jobs ADD COLUMN source_deleted_at TEXT;
ALTER TABLE import_jobs ADD COLUMN destination_mod_id TEXT REFERENCES mods(id) ON DELETE SET NULL;

UPDATE import_jobs SET source_group_id = id WHERE source_group_id IS NULL;

CREATE INDEX idx_import_jobs_mod_inbox_history
    ON import_jobs(game_id, source_processed_at DESC)
    WHERE source_processed_at IS NOT NULL;

CREATE INDEX idx_import_jobs_source_group
    ON import_jobs(source_group_id);
