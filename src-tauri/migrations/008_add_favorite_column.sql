-- Epic 10: Add favorite support to mods table
ALTER TABLE mods ADD COLUMN is_favorite BOOLEAN DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_mods_favorite ON mods(game_id, is_favorite);
