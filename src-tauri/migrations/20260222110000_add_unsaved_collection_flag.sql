ALTER TABLE collections ADD COLUMN is_last_unsaved BOOLEAN DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_collections_unsaved ON collections(game_id, is_last_unsaved);
