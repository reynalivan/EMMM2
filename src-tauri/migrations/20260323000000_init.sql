-- ==============================================================================
-- EMMM Master Database Schema
-- ==============================================================================

-- ------------------------------------------------------------------------------
-- 0. PRAGMA & ENGINE CONFIGURATION
-- ------------------------------------------------------------------------------
PRAGMA foreign_keys = ON;
PRAGMA auto_vacuum = INCREMENTAL;

-- ------------------------------------------------------------------------------
-- 1. SETTINGS & METADATA
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
) STRICT;

CREATE TABLE IF NOT EXISTS browser_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

INSERT OR IGNORE INTO browser_settings (key, value) VALUES
    ('homepage_url',        'https://www.google.com'),
    ('auto_import',         'true'),
    ('skip_picker_single',  'true'),
    ('allowed_extensions',  '.zip,.7z,.rar,.tar,.gz'),
    ('retention_days',      '3');

CREATE TABLE IF NOT EXISTS browser_downloads (
    id TEXT PRIMARY KEY,
    filename TEXT NOT NULL,
    status TEXT NOT NULL,
    total_bytes INTEGER,
    received_bytes INTEGER,
    progress REAL,
    error_message TEXT,
    file_path TEXT,
    session_id TEXT,
    started_at TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
) STRICT;

CREATE TABLE IF NOT EXISTS download_sessions (
    id TEXT PRIMARY KEY,
    game_id TEXT,
    status TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE SET NULL
) STRICT;

CREATE TABLE IF NOT EXISTS metadata (
    key TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    payload TEXT NOT NULL CHECK(json_valid(payload)),
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
) STRICT;

-- ------------------------------------------------------------------------------
-- 2. CORE ENTITIES (Games, Objects, Mods)
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS games (
    id TEXT PRIMARY KEY,
    game_type INTEGER NOT NULL, -- 0: GIMI, 1: SRMI, 2: WWMI, 3: ZZMI, 4: EFMI
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    mods_path TEXT NOT NULL,
    game_exe TEXT,
    launcher_path TEXT,
    loader_exe TEXT,
    launch_args TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(game_type, path)
) STRICT;

CREATE TABLE IF NOT EXISTS objects (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    name TEXT NOT NULL,
    name_key TEXT,
    folder_path TEXT NOT NULL,
    folder_path_key TEXT,
    status INTEGER NOT NULL DEFAULT 1 CHECK(status IN (0, 1)), -- 1: ENABLED, 0: DISABLED
    object_type TEXT,
    sub_category TEXT,
    is_pinned INTEGER NOT NULL DEFAULT 0,
    is_auto_sync INTEGER CHECK(is_auto_sync IN (0, 1)),
    tags TEXT CHECK(tags IS NULL OR json_valid(tags)),
    metadata TEXT CHECK(metadata IS NULL OR json_valid(metadata)),
    hash_db TEXT CHECK(hash_db IS NULL OR json_valid(hash_db)),
    custom_skins TEXT CHECK(custom_skins IS NULL OR json_valid(custom_skins)),
    thumbnail_path TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE,
    UNIQUE(game_id, name COLLATE NOCASE)
) STRICT;

CREATE TABLE IF NOT EXISTS mods (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    object_id TEXT,
    folder_path TEXT NOT NULL,
    folder_path_key TEXT NOT NULL,
    actual_name TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 0 CHECK(status IN (0, 1)), -- 1: ENABLED, 0: DISABLED
    object_type TEXT,
    disabled_reason TEXT,
    is_safe INTEGER NOT NULL DEFAULT 1,
    is_favorite INTEGER NOT NULL DEFAULT 0,
    is_pinned INTEGER NOT NULL DEFAULT 0,
    corridor_source TEXT NOT NULL DEFAULT 'unknown',
    content_hash TEXT, 
    size_bytes INTEGER NOT NULL DEFAULT 0,
    indexed_at TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY(object_id) REFERENCES objects(id) ON DELETE CASCADE,
    UNIQUE(game_id, folder_path COLLATE NOCASE)
) STRICT;

-- ------------------------------------------------------------------------------
-- 3. VIRTUAL COLLECTIONS & CORRIDOR STATE
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    name TEXT NOT NULL,
    name_key TEXT,
    is_safe INTEGER NOT NULL,
    is_unsaved INTEGER NOT NULL DEFAULT 0,
    is_last_unsaved INTEGER NOT NULL DEFAULT 0,
    last_active INTEGER NOT NULL DEFAULT 0,
    snapshot_json TEXT CHECK(snapshot_json IS NULL OR json_valid(snapshot_json)),
    signature TEXT,
    root_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
) STRICT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_one_unsaved_per_corridor 
ON collections (game_id, is_safe) WHERE is_unsaved = 1;

CREATE TABLE IF NOT EXISTS corridor_state (
    game_id TEXT NOT NULL,
    is_safe INTEGER NOT NULL CHECK(is_safe IN (0, 1)),
    active_collection_id TEXT REFERENCES collections(id) ON DELETE SET NULL,
    undo_collection_id TEXT REFERENCES collections(id) ON DELETE SET NULL,
    PRIMARY KEY (game_id, is_safe),
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE IF NOT EXISTS corridor_runtime_cache (
    game_id TEXT NOT NULL,
    is_safe INTEGER NOT NULL CHECK(is_safe IN (0, 1)),
    matched_collection_id TEXT REFERENCES collections(id) ON DELETE SET NULL,
    state_kind TEXT NOT NULL,
    state_name TEXT,
    signature TEXT NOT NULL,
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)),
    snapshot_source TEXT NOT NULL,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (game_id, is_safe),
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE IF NOT EXISTS collection_mods (
    collection_id TEXT NOT NULL,
    mod_id TEXT, 
    mod_path TEXT NOT NULL,
    mod_path_key TEXT,
    object_id TEXT NOT NULL,
    PRIMARY KEY (collection_id, mod_path),
    FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY(mod_id) REFERENCES mods(id) ON DELETE SET NULL,
    FOREIGN KEY(object_id) REFERENCES objects(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE IF NOT EXISTS collection_objects (
    collection_id TEXT NOT NULL,
    object_id TEXT NOT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (collection_id, object_id),
    FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY(object_id) REFERENCES objects(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE IF NOT EXISTS collection_nested_items (
    collection_id TEXT NOT NULL,
    mod_path_key TEXT NOT NULL,
    PRIMARY KEY (collection_id, mod_path_key),
    FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE IF NOT EXISTS collection_roots (
    collection_id TEXT NOT NULL,
    root_path TEXT NOT NULL,
    root_path_key TEXT NOT NULL,
    display_name TEXT NOT NULL,
    display_name_key TEXT NOT NULL,
    object_id TEXT,
    object_name TEXT,
    object_type TEXT,
    root_kind TEXT NOT NULL,
    is_safe INTEGER NOT NULL DEFAULT 1,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    thumbnail_hint TEXT,
    corridor_source TEXT,
    PRIMARY KEY (collection_id, root_path_key),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY (object_id) REFERENCES objects(id) ON DELETE SET NULL
) STRICT;

CREATE TABLE IF NOT EXISTS collection_signatures (
    collection_id TEXT PRIMARY KEY,
    signature TEXT NOT NULL,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
) STRICT;

-- ------------------------------------------------------------------------------
-- 4. TASKS, JOBS & WORKERS
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    task_type TEXT NOT NULL,
    target_id TEXT,
    payload TEXT CHECK(payload IS NULL OR json_valid(payload)),
    status TEXT NOT NULL DEFAULT 'PENDING',
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE IF NOT EXISTS import_jobs (
    id TEXT PRIMARY KEY,
    download_id TEXT,
    source_url TEXT,
    archive_path TEXT,
    archive_hash TEXT,
    archive_size INTEGER,
    game_id TEXT REFERENCES games(id) ON DELETE SET NULL,
    staging_path TEXT,
    status TEXT NOT NULL DEFAULT 'queued',
    match_category TEXT,
    match_object_id TEXT,
    match_confidence REAL,
    match_reason TEXT,
    placed_path TEXT,
    error_msg TEXT,
    is_duplicate INTEGER NOT NULL DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
) STRICT;

-- ------------------------------------------------------------------------------
-- 5. KEYVIEWER
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS mod_hash_index (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    mod_id TEXT NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    hash TEXT NOT NULL COLLATE NOCASE,
    section_name TEXT NOT NULL DEFAULT '',
    file_path TEXT NOT NULL DEFAULT '',
    collision_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
) STRICT;

CREATE TABLE IF NOT EXISTS object_sentinel_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    object_id TEXT NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    sentinel_hash TEXT NOT NULL COLLATE NOCASE,
    confidence REAL NOT NULL,
    is_manual INTEGER NOT NULL DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
) STRICT;

CREATE TABLE IF NOT EXISTS game_sentinel_settings (
    game_id TEXT PRIMARY KEY REFERENCES games(id) ON DELETE CASCADE,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    overlay_position TEXT NOT NULL DEFAULT 'top_left',
    display_mode TEXT NOT NULL DEFAULT 'auto'
) STRICT;

-- ------------------------------------------------------------------------------
-- 6. DEDUP SCANNER
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS dedup_jobs (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed', 'canceled')),
    started_at TEXT DEFAULT CURRENT_TIMESTAMP,
    completed_at TEXT
) STRICT;

CREATE TABLE IF NOT EXISTS dedup_groups (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES dedup_jobs(id) ON DELETE CASCADE,
    reasons_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(reasons_json)),
    resolution_status TEXT NOT NULL DEFAULT 'pending' CHECK (resolution_status IN ('pending', 'resolved', 'ignored', 'partial')),
    resolved_at TEXT,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
) STRICT;

CREATE TABLE IF NOT EXISTS dedup_group_members (
    id TEXT PRIMARY KEY,
    group_id TEXT NOT NULL REFERENCES dedup_groups(id) ON DELETE CASCADE,
    folder_id TEXT NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    file_hash TEXT,
    signals_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(signals_json)),
    is_primary INTEGER NOT NULL DEFAULT 0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (group_id, folder_id)
) STRICT;

CREATE TABLE IF NOT EXISTS duplicate_whitelist (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    folder_a_id TEXT NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    folder_b_id TEXT NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    ignored_at TEXT DEFAULT CURRENT_TIMESTAMP,
    reason TEXT,
    UNIQUE (game_id, folder_a_id, folder_b_id)
) STRICT;

-- ------------------------------------------------------------------------------
-- 7. PERFORMANCE VIEWS
-- ------------------------------------------------------------------------------
CREATE VIEW IF NOT EXISTS v_object_mod_stats AS
SELECT 
    object_id,
    game_id,
    is_safe,
    COUNT(id) AS total_mods,
    SUM(CASE WHEN status = 1 THEN 1 ELSE 0 END) AS enabled_mods
FROM mods
GROUP BY object_id, game_id, is_safe;

-- ------------------------------------------------------------------------------
-- 8. PERFORMANCE INDEXES
-- ------------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_mods_object_id   ON mods(object_id);
CREATE INDEX IF NOT EXISTS idx_mods_game_status ON mods(game_id, status);
CREATE INDEX IF NOT EXISTS idx_mods_game_safe   ON mods(game_id, is_safe);
CREATE INDEX IF NOT EXISTS idx_mods_folder_path_key ON mods(game_id, folder_path_key);

CREATE INDEX IF NOT EXISTS idx_objects_game_id  ON objects(game_id);
CREATE INDEX IF NOT EXISTS idx_objects_name_key ON objects(game_id, name_key);

CREATE INDEX IF NOT EXISTS idx_objects_type_sub ON objects(game_id, object_type, sub_category);

CREATE INDEX IF NOT EXISTS idx_collections_name_key_context ON collections(game_id, name_key, is_safe);
CREATE INDEX IF NOT EXISTS idx_collection_mods_collection_id ON collection_mods(collection_id);

CREATE INDEX IF NOT EXISTS idx_import_jobs_status   ON import_jobs(status);
CREATE INDEX IF NOT EXISTS idx_import_jobs_hash     ON import_jobs(archive_hash);
CREATE INDEX IF NOT EXISTS idx_tasks_status         ON tasks(status) WHERE status = 'PENDING';

CREATE INDEX IF NOT EXISTS idx_mod_hash_index_hash ON mod_hash_index(hash);
CREATE INDEX IF NOT EXISTS idx_mod_hash_index_game ON mod_hash_index(game_id);

CREATE INDEX IF NOT EXISTS idx_dedup_jobs_game     ON dedup_jobs(game_id);
CREATE INDEX IF NOT EXISTS idx_dedup_groups_job    ON dedup_groups(job_id);

-- ------------------------------------------------------------------------------
-- 9. AUTOMATIC 'updated_at' TRIGGERS
-- ------------------------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS trg_games_updated_at AFTER UPDATE ON games FOR EACH ROW BEGIN UPDATE games SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END;
CREATE TRIGGER IF NOT EXISTS trg_objects_updated_at AFTER UPDATE ON objects FOR EACH ROW BEGIN UPDATE objects SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END;
CREATE TRIGGER IF NOT EXISTS trg_mods_updated_at AFTER UPDATE ON mods FOR EACH ROW BEGIN UPDATE mods SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END;
CREATE TRIGGER IF NOT EXISTS trg_collections_updated_at AFTER UPDATE ON collections FOR EACH ROW BEGIN UPDATE collections SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END;
CREATE TRIGGER IF NOT EXISTS trg_import_jobs_updated_at AFTER UPDATE ON import_jobs FOR EACH ROW BEGIN UPDATE import_jobs SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END;

-- indexed_at: set on first INSERT only (never overwrite)
CREATE TRIGGER IF NOT EXISTS trg_mods_indexed_at AFTER INSERT ON mods FOR EACH ROW WHEN NEW.indexed_at IS NULL BEGIN UPDATE mods SET indexed_at = CURRENT_TIMESTAMP WHERE id = NEW.id; END;
