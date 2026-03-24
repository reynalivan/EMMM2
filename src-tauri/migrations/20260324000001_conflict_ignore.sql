-- Migration: 20260324000001_conflict_ignore.sql
-- Description: Create persistent storage for ignored object-level conflicts.

CREATE TABLE IF NOT EXISTS ignored_object_conflicts (
    id TEXT PRIMARY KEY NOT NULL, -- UUID string
    game_id TEXT NOT NULL,
    object_id TEXT NOT NULL,
    -- mod_ids is a JSON array of mod IDs (folder names/keys), sorted lexicographically
    mod_ids TEXT NOT NULL, 
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE,
    UNIQUE(game_id, object_id, mod_ids)
);

CREATE INDEX IF NOT EXISTS idx_ignored_conflicts_game ON ignored_object_conflicts(game_id);
CREATE INDEX IF NOT EXISTS idx_ignored_conflicts_lookup ON ignored_object_conflicts(game_id, object_id);
