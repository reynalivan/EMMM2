ALTER TABLE objects ADD COLUMN matched_entry_key TEXT;
ALTER TABLE objects ADD COLUMN matched_alias_name TEXT;
ALTER TABLE objects ADD COLUMN matched_confidence REAL;
ALTER TABLE objects ADD COLUMN matched_reason TEXT;
ALTER TABLE objects ADD COLUMN matched_source TEXT;
ALTER TABLE objects ADD COLUMN matched_at TEXT;

ALTER TABLE import_jobs ADD COLUMN match_entry_key TEXT;
ALTER TABLE import_jobs ADD COLUMN match_alias_name TEXT;

CREATE INDEX IF NOT EXISTS idx_objects_matched_entry_key
ON objects(game_id, matched_entry_key);
