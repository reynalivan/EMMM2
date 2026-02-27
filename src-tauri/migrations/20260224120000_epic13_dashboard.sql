-- Epic 13: Dashboard query support

-- Track when each mod was first indexed
ALTER TABLE mods ADD COLUMN indexed_at DATETIME;
UPDATE mods SET indexed_at = CURRENT_TIMESTAMP WHERE indexed_at IS NULL;

CREATE TRIGGER IF NOT EXISTS trg_mods_indexed_at
AFTER INSERT ON mods
FOR EACH ROW
WHEN NEW.indexed_at IS NULL
BEGIN
    UPDATE mods SET indexed_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;


-- Index for "Recently Added" query (ORDER BY indexed_at DESC LIMIT 5)
CREATE INDEX IF NOT EXISTS idx_mods_indexed_at ON mods(indexed_at);

-- Index for category distribution (GROUP BY object_type)
CREATE INDEX IF NOT EXISTS idx_mods_object_type ON mods(object_type);

-- Composite index for safe mode filtered aggregations
CREATE INDEX IF NOT EXISTS idx_mods_game_safe ON mods(game_id, is_safe);
