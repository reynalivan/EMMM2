-- Games table
CREATE TABLE IF NOT EXISTS games (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    game_type TEXT NOT NULL,
    path TEXT NOT NULL,
    launcher_path TEXT,
    launch_args TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Mods table
CREATE TABLE IF NOT EXISTS mods (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    actual_name TEXT NOT NULL,
    folder_path TEXT NOT NULL,
    status TEXT DEFAULT 'DISABLED',
    is_pinned BOOLEAN DEFAULT 0,
    is_safe BOOLEAN DEFAULT 0,
    last_status_active BOOLEAN,
    size_bytes INTEGER,
    object_type TEXT,
    metadata_blob JSON,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
);

-- Collections table
CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    game_id TEXT NOT NULL
);

-- Collection Items table
CREATE TABLE IF NOT EXISTS collection_items (
    collection_id TEXT NOT NULL,
    mod_id TEXT NOT NULL,
    FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY(mod_id) REFERENCES mods(id) ON DELETE CASCADE
);
