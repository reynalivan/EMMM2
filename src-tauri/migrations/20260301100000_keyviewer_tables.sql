-- KeyViewer tables for req-42: hash indexing, sentinel cache, keybind cache

-- Harvested hashes from active mods (index for fast lookup)
CREATE TABLE IF NOT EXISTS mod_hash_index (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id     INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    mod_id      INTEGER NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    hash        TEXT    NOT NULL COLLATE NOCASE,
    section_name TEXT   NOT NULL DEFAULT '',
    file_path   TEXT    NOT NULL DEFAULT '',
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_mod_hash_index_hash
    ON mod_hash_index(hash);
CREATE INDEX IF NOT EXISTS idx_mod_hash_index_game
    ON mod_hash_index(game_id);
CREATE INDEX IF NOT EXISTS idx_mod_hash_index_mod
    ON mod_hash_index(mod_id);

-- Selected sentinel hashes per object (for runtime detection)
CREATE TABLE IF NOT EXISTS object_sentinel_cache (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id     INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    object_name TEXT    NOT NULL,
    sentinel_hashes TEXT NOT NULL DEFAULT '[]',  -- JSON array of sentinel hash strings
    confidence  REAL    NOT NULL DEFAULT 0.0,
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE(game_id, object_name)
);

-- Extracted keybinds per object (for overlay text generation)
CREATE TABLE IF NOT EXISTS keybind_cache (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id     INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    object_name TEXT    NOT NULL,
    keybind_json TEXT   NOT NULL DEFAULT '[]',  -- JSON array of keybind objects
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE(game_id, object_name)
);

-- File signature cache for incremental scanning
CREATE TABLE IF NOT EXISTS file_signature_cache (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path   TEXT    NOT NULL UNIQUE,
    file_size   INTEGER NOT NULL DEFAULT 0,
    mtime_secs  INTEGER NOT NULL DEFAULT 0,
    updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);
