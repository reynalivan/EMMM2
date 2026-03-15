-- =============================================================================
-- BASELINE SCHEMA — Consolidated from migrations 001 through 20260315100000
-- This is the final canonical state of all core tables.
-- Generated: 2026-03-15
-- =============================================================================

-- ---------------------------------------------------------------------------
-- GAMES
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS games (
    id           TEXT PRIMARY KEY,
    name         TEXT NOT NULL,
    game_type    TEXT NOT NULL,
    path         TEXT NOT NULL,
    launcher_path TEXT,
    launch_args  TEXT,
    mod_path     TEXT,
    game_exe     TEXT,
    loader_exe   TEXT,
    created_at   DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_games_path ON games(path);

-- ---------------------------------------------------------------------------
-- MODS
-- NOTE: is_safe lives here (mod-level), NOT on objects.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS mods (
    id                  TEXT PRIMARY KEY,
    game_id             TEXT NOT NULL,
    actual_name         TEXT NOT NULL,
    folder_path         TEXT NOT NULL,
    status              TEXT DEFAULT 'DISABLED',
    is_pinned           BOOLEAN DEFAULT 0,
    is_safe             BOOLEAN DEFAULT 1,
    is_favorite         BOOLEAN DEFAULT 0,
    last_status_active  BOOLEAN,
    last_status_sfw     BOOLEAN,
    last_status_nsfw    BOOLEAN,
    size_bytes          INTEGER,
    object_type         TEXT,
    object_id           TEXT REFERENCES objects(id) ON DELETE SET NULL,
    indexed_at          DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at          DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
);

-- ---------------------------------------------------------------------------
-- OBJECTS
-- NOTE: is_safe was intentionally removed — safety is at the mod level only.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS objects (
    id           TEXT PRIMARY KEY,
    game_id      TEXT NOT NULL,
    name         TEXT NOT NULL,
    folder_path  TEXT,
    object_type  TEXT NOT NULL DEFAULT 'Other',
    sub_category TEXT,
    sort_order   INTEGER DEFAULT 0,
    tags         JSON DEFAULT '[]',
    metadata     JSON DEFAULT '{}',
    thumbnail_path TEXT,
    is_pinned    BOOLEAN DEFAULT 0,
    is_auto_sync BOOLEAN NOT NULL DEFAULT 0,
    created_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at   DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
);

-- Backfill: ensure all objects have folder_path = name (legacy assumption)
UPDATE objects SET folder_path = name WHERE folder_path IS NULL;

-- ---------------------------------------------------------------------------
-- COLLECTIONS
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS collections (
    id               TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    game_id          TEXT NOT NULL,
    is_safe_context  BOOLEAN DEFAULT 0,
    is_last_unsaved  BOOLEAN DEFAULT 0,
    updated_at       DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- ---------------------------------------------------------------------------
-- COLLECTION ITEMS
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS collection_items (
    collection_id TEXT NOT NULL,
    mod_id        TEXT NOT NULL,
    mod_path      TEXT,                          -- fallback for resilience when mod IDs change
    FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY(mod_id) REFERENCES mods(id) ON DELETE CASCADE
);

-- ---------------------------------------------------------------------------
-- COLLECTION NESTED ITEMS (nested mod folder snapshots)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS collection_nested_items (
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    mod_path      TEXT NOT NULL,
    PRIMARY KEY (collection_id, mod_path)
);

-- ---------------------------------------------------------------------------
-- SCAN RESULTS (transient staging; cleared after scan confirm)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS scan_results (
    id              TEXT PRIMARY KEY,
    game_id         TEXT NOT NULL,
    source_path     TEXT NOT NULL,
    source_name     TEXT NOT NULL,
    matched_object  TEXT,
    object_type     TEXT,
    match_level     TEXT,
    match_confidence TEXT,
    match_detail    TEXT,
    detected_skin   TEXT,
    thumbnail_path  TEXT,
    status          TEXT DEFAULT 'PENDING',
    error_message   TEXT,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
);

-- ---------------------------------------------------------------------------
-- APP SETTINGS (general key-value config)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- ---------------------------------------------------------------------------
-- APP META (sync state tracking)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS app_meta (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO app_meta (key, value) VALUES ('metadata_version', '0');
INSERT OR IGNORE INTO app_meta (key, value) VALUES ('last_modified', '');
INSERT OR IGNORE INTO app_meta (key, value) VALUES ('etag', '');

-- ---------------------------------------------------------------------------
-- DEDUP SCANNER
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS dedup_jobs (
    id                       TEXT PRIMARY KEY,
    game_id                  TEXT NOT NULL,
    started_at               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at             DATETIME,
    status                   TEXT NOT NULL DEFAULT 'in_progress'
                                 CHECK (status IN ('in_progress', 'completed', 'cancelled')),
    scanned_mods_count       INTEGER NOT NULL DEFAULT 0,
    duplicate_groups_count   INTEGER NOT NULL DEFAULT 0,
    duplicate_members_count  INTEGER NOT NULL DEFAULT 0,
    stats_json               JSON NOT NULL DEFAULT '{}',
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS dedup_groups (
    id                 TEXT PRIMARY KEY,
    job_id             TEXT NOT NULL,
    confidence         INTEGER NOT NULL DEFAULT 0 CHECK (confidence >= 0 AND confidence <= 100),
    primary_signal     TEXT NOT NULL,
    match_reasons_json JSON NOT NULL DEFAULT '[]',
    resolution_status  TEXT NOT NULL DEFAULT 'pending'
                           CHECK (resolution_status IN ('pending', 'resolved', 'ignored', 'partial')),
    resolved_at        DATETIME,
    created_at         DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (job_id) REFERENCES dedup_jobs(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS dedup_group_members (
    id          TEXT PRIMARY KEY,
    group_id    TEXT NOT NULL,
    folder_id   TEXT NOT NULL,
    file_hash   TEXT,
    signals_json JSON NOT NULL DEFAULT '{}',
    is_primary  BOOLEAN NOT NULL DEFAULT 0,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (group_id) REFERENCES dedup_groups(id) ON DELETE CASCADE,
    FOREIGN KEY (folder_id) REFERENCES mods(id) ON DELETE CASCADE,
    UNIQUE (group_id, folder_id)
);

CREATE TABLE IF NOT EXISTS duplicate_whitelist (
    id           TEXT PRIMARY KEY,
    game_id      TEXT NOT NULL,
    folder_a_id  TEXT NOT NULL,
    folder_b_id  TEXT NOT NULL,
    ignored_at   DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reason       TEXT,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (folder_a_id) REFERENCES mods(id) ON DELETE CASCADE,
    FOREIGN KEY (folder_b_id) REFERENCES mods(id) ON DELETE CASCADE,
    CHECK (folder_a_id <> folder_b_id),
    CHECK (folder_a_id < folder_b_id),
    UNIQUE (game_id, folder_a_id, folder_b_id)
);
