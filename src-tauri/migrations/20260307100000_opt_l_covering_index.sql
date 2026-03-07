-- Opt-L: Covering index for default ObjectList sort query
-- Covers: WHERE game_id = ? ... ORDER BY is_pinned DESC, object_type, name ASC
CREATE INDEX IF NOT EXISTS idx_objects_default_sort
  ON objects(game_id, is_pinned DESC, object_type, name);
