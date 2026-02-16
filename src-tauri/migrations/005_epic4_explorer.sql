-- Epic 4: Explorer tables and indexes
-- Covers: US-4.4 (Trash), DI-4.01

-- Trash log for tracking deleted items and enabling restore
CREATE TABLE IF NOT EXISTS trash_log (
    id TEXT PRIMARY KEY,
    original_path TEXT NOT NULL,
    original_name TEXT NOT NULL,
    game_id TEXT,
    deleted_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    size_bytes INTEGER DEFAULT 0,
    metadata JSON,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE SET NULL
);

-- Performance indexes for mod queries in the explorer
CREATE INDEX IF NOT EXISTS idx_mods_game_status ON mods(game_id, status);
CREATE INDEX IF NOT EXISTS idx_mods_pinned ON mods(game_id, is_pinned);
CREATE INDEX IF NOT EXISTS idx_trash_game ON trash_log(game_id);
