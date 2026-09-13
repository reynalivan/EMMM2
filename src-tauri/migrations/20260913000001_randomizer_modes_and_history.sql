ALTER TABLE objects ADD COLUMN randomizer_mode TEXT;

CREATE TABLE IF NOT EXISTS randomizer_applied_history (
    game_id TEXT NOT NULL,
    object_id TEXT NOT NULL,
    mod_id TEXT NOT NULL,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (game_id, object_id, mod_id),
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (object_id) REFERENCES objects(id) ON DELETE CASCADE,
    FOREIGN KEY (mod_id) REFERENCES mods(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX IF NOT EXISTS idx_randomizer_applied_history_recent
    ON randomizer_applied_history(game_id, object_id, applied_at DESC);
