-- =============================================================================
-- FEATURE TABLES — KeyViewer (req-42) and Discover Hub / In-App Browser (epic-44)
-- These tables have no dependencies on older ALTER TABLE chains.
-- Generated: 2026-03-15
-- =============================================================================

-- ---------------------------------------------------------------------------
-- KEYVIEWER — Hash index, sentinel cache, keybind cache, file sig cache
-- ---------------------------------------------------------------------------

-- Harvested hashes from active mods (index for fast lookup)
CREATE TABLE IF NOT EXISTS mod_hash_index (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id      TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    mod_id       TEXT NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    hash         TEXT NOT NULL COLLATE NOCASE,
    section_name TEXT NOT NULL DEFAULT '',
    file_path    TEXT NOT NULL DEFAULT '',
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_mod_hash_index_hash ON mod_hash_index(hash);
CREATE INDEX IF NOT EXISTS idx_mod_hash_index_game ON mod_hash_index(game_id);
CREATE INDEX IF NOT EXISTS idx_mod_hash_index_mod  ON mod_hash_index(mod_id);

-- Selected sentinel hashes per object (for runtime detection)
CREATE TABLE IF NOT EXISTS object_sentinel_cache (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id          TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    object_name      TEXT NOT NULL,
    sentinel_hashes  TEXT NOT NULL DEFAULT '[]',   -- JSON array of sentinel hash strings
    confidence       REAL NOT NULL DEFAULT 0.0,
    updated_at       TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(game_id, object_name)
);

-- Extracted keybinds per object (for overlay text generation)
CREATE TABLE IF NOT EXISTS keybind_cache (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id      TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    object_name  TEXT NOT NULL,
    keybind_json TEXT NOT NULL DEFAULT '[]',        -- JSON array of keybind objects
    updated_at   TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(game_id, object_name)
);

-- File signature cache for incremental scanning
CREATE TABLE IF NOT EXISTS file_signature_cache (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path  TEXT NOT NULL UNIQUE,
    file_size  INTEGER NOT NULL DEFAULT 0,
    mtime_secs INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ---------------------------------------------------------------------------
-- DISCOVER HUB / IN-APP BROWSER (epic-44)
-- ---------------------------------------------------------------------------

-- Download Sessions: correlates a Discover Hub "Download" click to intercepted files
CREATE TABLE IF NOT EXISTS download_sessions (
    id                TEXT PRIMARY KEY,                -- UUID v4
    source            TEXT NOT NULL,                  -- 'gamebanana' | 'adhoc'
    submission_id     TEXT,
    mod_title         TEXT,
    profile_url       TEXT,
    game_id           TEXT REFERENCES games(id) ON DELETE SET NULL,
    expected_keywords TEXT,                           -- JSON array of token strings
    status            TEXT NOT NULL DEFAULT 'awaiting_download',
    created_at        TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Browser Downloads: one row per intercepted file download
CREATE TABLE IF NOT EXISTS browser_downloads (
    id             TEXT PRIMARY KEY,                  -- UUID v4
    session_id     TEXT REFERENCES download_sessions(id) ON DELETE SET NULL,
    filename       TEXT NOT NULL,
    file_path      TEXT,                              -- full path (set on Finished)
    source_url     TEXT,
    status         TEXT NOT NULL DEFAULT 'requested',
    -- Values: requested | in_progress | finished | failed | canceled | imported
    bytes_total    INTEGER,
    bytes_received INTEGER NOT NULL DEFAULT 0,
    error_msg      TEXT,
    started_at     TEXT NOT NULL DEFAULT (datetime('now')),
    finished_at    TEXT
);

CREATE INDEX IF NOT EXISTS idx_browser_downloads_status  ON browser_downloads(status);
CREATE INDEX IF NOT EXISTS idx_browser_downloads_session ON browser_downloads(session_id);

-- Import Jobs: tracks the Smart Import pipeline for each downloaded file
CREATE TABLE IF NOT EXISTS import_jobs (
    id               TEXT PRIMARY KEY,                -- UUID v4
    download_id      TEXT REFERENCES browser_downloads(id) ON DELETE SET NULL,
    game_id          TEXT REFERENCES games(id) ON DELETE SET NULL,
    archive_path     TEXT NOT NULL,                   -- original file in BrowserDownloadsRoot
    archive_hash     TEXT,                            -- BLAKE3 hex for deduplication
    staging_path     TEXT,                            -- temp extraction path
    status           TEXT NOT NULL DEFAULT 'queued',
    -- Values: queued | extracting | matching | needs_review | placing | done | failed | canceled
    match_category   TEXT,
    match_object_id  TEXT,
    match_confidence REAL,
    match_reason     TEXT,
    placed_path      TEXT,                            -- final folder path in workspace
    error_msg        TEXT,
    is_duplicate     INTEGER NOT NULL DEFAULT 0,      -- 1 if archive_hash already existed
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_import_jobs_status   ON import_jobs(status);
CREATE INDEX IF NOT EXISTS idx_import_jobs_download ON import_jobs(download_id);
CREATE INDEX IF NOT EXISTS idx_import_jobs_hash     ON import_jobs(archive_hash);

-- Browser Settings: key-value store for browser & import configuration
CREATE TABLE IF NOT EXISTS browser_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO browser_settings (key, value) VALUES
    ('homepage_url',        'https://www.google.com'),
    ('auto_import',         'true'),
    ('skip_picker_single',  'true'),
    ('allowed_extensions',  '.zip,.7z,.rar,.tar,.gz'),
    ('retention_days',      '30'),
    ('downloads_root',      '');
    -- empty string means use AppData default: AppData/EMM2/BrowserDownloads/
