CREATE TABLE IF NOT EXISTS ignored_active_mod_conflict_groups (
    game_id TEXT NOT NULL,
    mod_paths TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,

    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    UNIQUE (game_id, mod_paths)
);

CREATE INDEX IF NOT EXISTS idx_ignored_active_mod_conflicts_game
    ON ignored_active_mod_conflict_groups (game_id);
