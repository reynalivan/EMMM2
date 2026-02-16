-- Epic 4: Add pinning support to objects table
ALTER TABLE objects ADD COLUMN is_pinned BOOLEAN DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_objects_pinned ON objects(game_id, is_pinned);
