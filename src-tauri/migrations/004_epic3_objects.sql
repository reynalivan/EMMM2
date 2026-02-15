-- Epic 3: Objects table for categorized game entities (Character, Weapon, UI, Other)
CREATE TABLE IF NOT EXISTS objects (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    name TEXT NOT NULL,
    object_type TEXT NOT NULL DEFAULT 'Other',
    sub_category TEXT,
    sort_order INTEGER DEFAULT 0,
    tags JSON DEFAULT '[]',
    metadata JSON DEFAULT '{}',
    thumbnail_path TEXT,
    is_safe BOOLEAN DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
);

-- Performance indexes
CREATE INDEX IF NOT EXISTS idx_objects_game_id ON objects(game_id);
CREATE INDEX IF NOT EXISTS idx_objects_game_type ON objects(game_id, object_type);
CREATE UNIQUE INDEX IF NOT EXISTS idx_objects_game_name ON objects(game_id, name);

-- Link mods to objects (nullable FK for uncategorized mods)
ALTER TABLE mods ADD COLUMN object_id TEXT REFERENCES objects(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_mods_object_id ON mods(object_id);
