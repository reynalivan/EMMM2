-- =========================================
-- MIGRATION: 20260323000000_init.sql
-- =========================================

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
    is_safe INTEGER NOT NULL DEFAULT 1,
    is_favorite INTEGER NOT NULL DEFAULT 0,
    is_pinned INTEGER NOT NULL DEFAULT 0,
    safety_source TEXT NOT NULL DEFAULT 'unknown',
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
-- 3. COLLECTION SNAPSHOTS
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    name TEXT NOT NULL,
    name_key TEXT,
    is_safe INTEGER NOT NULL,
    snapshot_json TEXT CHECK(snapshot_json IS NULL OR json_valid(snapshot_json)),
    signature TEXT,
    display_mod_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE IF NOT EXISTS collection_mods (
    collection_id TEXT NOT NULL,
    mod_id TEXT,
    mod_path TEXT NOT NULL,
    mod_path_key TEXT,
    object_ref_key TEXT NOT NULL,
    object_id TEXT,
    preview_path TEXT,
    node_type TEXT,
    warnings_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(warnings_json)),
    is_safe INTEGER NOT NULL DEFAULT 1,
    safety_source TEXT,
    PRIMARY KEY (collection_id, mod_path),
    FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY(mod_id) REFERENCES mods(id) ON DELETE SET NULL,
    FOREIGN KEY(object_id) REFERENCES objects(id) ON DELETE SET NULL
) STRICT;

CREATE TABLE IF NOT EXISTS collection_objects (
    collection_id TEXT NOT NULL,
    object_ref_key TEXT NOT NULL,
    object_id TEXT,
    object_display_name TEXT,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (collection_id, object_ref_key),
    FOREIGN KEY(collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY(object_id) REFERENCES objects(id) ON DELETE SET NULL
) STRICT;

CREATE TABLE IF NOT EXISTS collection_runtime_state (
    game_id TEXT PRIMARY KEY,
    active_collection_id TEXT REFERENCES collections(id) ON DELETE SET NULL,
    draft_collection_id TEXT UNIQUE REFERENCES collections(id) ON DELETE SET NULL,
    draft_base_collection_id TEXT REFERENCES collections(id) ON DELETE SET NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
) STRICT;

-- ------------------------------------------------------------------------------
-- 4. TASKS, JOBS & WORKERS
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    task_type TEXT NOT NULL,
    target_id TEXT,
    rollback_collection_id TEXT,
    rollback_active_collection_id TEXT,
    final_active_collection_id TEXT,
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

CREATE UNIQUE INDEX IF NOT EXISTS idx_collections_named_name_key_game
    ON collections(game_id, name_key);
CREATE INDEX IF NOT EXISTS idx_collection_mods_collection_id ON collection_mods(collection_id);

CREATE INDEX IF NOT EXISTS idx_import_jobs_status   ON import_jobs(status);
CREATE INDEX IF NOT EXISTS idx_import_jobs_hash     ON import_jobs(archive_hash);
CREATE INDEX IF NOT EXISTS idx_tasks_status         ON tasks(status) WHERE status IN ('PENDING', 'RUNNING');
CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_one_open_collection_apply_per_game
    ON tasks(game_id)
    WHERE task_type = 'apply_collection' AND status IN ('PENDING', 'RUNNING');

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

-- =========================================
-- MIGRATION: 20260324000000_scrub_json_columns.sql
-- =========================================

-- Update corrupted JSON payloads in objects
UPDATE objects SET hash_db = NULL WHERE hash_db IS NOT NULL AND json_valid(hash_db) = 0;
UPDATE objects SET custom_skins = NULL WHERE custom_skins IS NOT NULL AND json_valid(custom_skins) = 0;

-- =========================================
-- MIGRATION: 20260324000001_conflict_ignore.sql
-- =========================================

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

-- =========================================
-- MIGRATION: 20260325220000_deepmatch_canonical_relation.sql
-- =========================================

ALTER TABLE objects ADD COLUMN matched_entry_key TEXT;
ALTER TABLE objects ADD COLUMN matched_alias_name TEXT;
ALTER TABLE objects ADD COLUMN matched_confidence REAL;
ALTER TABLE objects ADD COLUMN matched_reason TEXT;
ALTER TABLE objects ADD COLUMN matched_source TEXT;
ALTER TABLE objects ADD COLUMN matched_at TEXT;

ALTER TABLE import_jobs ADD COLUMN match_entry_key TEXT;
ALTER TABLE import_jobs ADD COLUMN match_alias_name TEXT;

CREATE INDEX IF NOT EXISTS idx_objects_matched_entry_key
ON objects(game_id, matched_entry_key);

-- =========================================
-- MIGRATION: 20260329133000_object_runtime_projection.sql
-- =========================================

-- Migration: 20260329133000_object_runtime_projection.sql
-- Purpose: projection-backed runtime counts/status for workspace/object read models

CREATE TABLE IF NOT EXISTS object_runtime_projection (
    game_id TEXT NOT NULL,
    object_id TEXT NOT NULL,
    object_type TEXT,
    mod_count_safe INTEGER NOT NULL DEFAULT 0,
    mod_count_unsafe INTEGER NOT NULL DEFAULT 0,
    enabled_count_safe INTEGER NOT NULL DEFAULT 0,
    enabled_count_unsafe INTEGER NOT NULL DEFAULT 0,
    is_object_disabled INTEGER NOT NULL DEFAULT 0 CHECK(is_object_disabled IN (0, 1)),
    has_naming_conflict INTEGER NOT NULL DEFAULT 0 CHECK(has_naming_conflict IN (0, 1)),
    active_mod_paths_safe_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(active_mod_paths_safe_json)),
    active_mod_paths_unsafe_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(active_mod_paths_unsafe_json)),
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (game_id, object_id),
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (object_id) REFERENCES objects(id) ON DELETE CASCADE
) STRICT;

CREATE INDEX IF NOT EXISTS idx_object_runtime_projection_game_object
ON object_runtime_projection(game_id, object_id);

CREATE INDEX IF NOT EXISTS idx_object_runtime_projection_game_type
ON object_runtime_projection(game_id, object_type);

CREATE INDEX IF NOT EXISTS idx_object_runtime_projection_game_conflict
ON object_runtime_projection(game_id, has_naming_conflict);

INSERT OR REPLACE INTO object_runtime_projection (
    game_id,
    object_id,
    object_type,
    mod_count_safe,
    mod_count_unsafe,
    enabled_count_safe,
    enabled_count_unsafe,
    is_object_disabled,
    has_naming_conflict,
    active_mod_paths_safe_json,
    active_mod_paths_unsafe_json,
    updated_at
)
SELECT
    o.game_id,
    o.id,
    o.object_type,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND (
            COALESCE(m.is_safe, 1) = 1
            OR COALESCE(m.safety_source, 'unknown') IN ('manual', 'unknown')
          )
    ) AS mod_count_safe,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND (
            COALESCE(m.is_safe, 1) = 0
            OR COALESCE(m.safety_source, 'unknown') IN ('manual', 'unknown')
          )
    ) AS mod_count_unsafe,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            COALESCE(m.is_safe, 1) = 1
            OR COALESCE(m.safety_source, 'unknown') IN ('manual', 'unknown')
          )
    ) AS enabled_count_safe,
    (
        SELECT COUNT(*)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            COALESCE(m.is_safe, 1) = 0
            OR COALESCE(m.safety_source, 'unknown') IN ('manual', 'unknown')
          )
    ) AS enabled_count_unsafe,
    CASE
        WHEN o.folder_path LIKE 'DISABLED %'
          OR o.folder_path LIKE '%/DISABLED %'
          OR o.folder_path LIKE '%\\DISABLED %'
        THEN 1
        ELSE 0
    END AS is_object_disabled,
    0 AS has_naming_conflict,
    COALESCE((
        SELECT json_group_array(m.folder_path)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            COALESCE(m.is_safe, 1) = 1
            OR COALESCE(m.safety_source, 'unknown') IN ('manual', 'unknown')
          )
    ), '[]') AS active_mod_paths_safe_json,
    COALESCE((
        SELECT json_group_array(m.folder_path)
        FROM mods m
        WHERE m.object_id = o.id
          AND m.status = 1
          AND (
            COALESCE(m.is_safe, 1) = 0
            OR COALESCE(m.safety_source, 'unknown') IN ('manual', 'unknown')
          )
    ), '[]') AS active_mod_paths_unsafe_json,
    CURRENT_TIMESTAMP
FROM objects o;

-- =========================================
-- MIGRATION: 20260406000000_fix_browser_downloads_schema.sql
-- =========================================

-- The init migration created browser_downloads with a schema that never
-- matched the service code (missing source_url/finished_at, wrong byte
-- column names), so every download insert failed at runtime. This table is
-- a transient download cache, so recreate it to match the code.
DROP TABLE IF EXISTS browser_downloads;

CREATE TABLE browser_downloads (
    id             TEXT PRIMARY KEY,
    session_id     TEXT,
    filename       TEXT NOT NULL,
    file_path      TEXT,
    source_url     TEXT,
    status         TEXT NOT NULL,
    bytes_received INTEGER NOT NULL DEFAULT 0,
    bytes_total    INTEGER,
    error_msg      TEXT,
    started_at     TEXT NOT NULL,
    finished_at    TEXT
) STRICT;

CREATE INDEX IF NOT EXISTS idx_browser_downloads_started_at
    ON browser_downloads (started_at DESC);

-- =========================================
-- MIGRATION: 20260810000000_hot_path_indexes.sql
-- =========================================

-- Indexes for predicates that were falling back to full table scans.
--
-- `idx_mods_folder_path_key` is on (game_id, folder_path_key), so a predicate
-- that omits `game_id` cannot use it. `batch_update_path_and_status` and
-- `batch_delete_by_path` match on the key alone, once per row inside a loop —
-- a 300-mod bulk operation over a 10k library was 3M row visits.
CREATE INDEX IF NOT EXISTS idx_mods_folder_path_key_only
    ON mods(folder_path_key);

-- `ensure_object_exists` and the object lookups match on
-- (game_id, folder_path_key), which had no supporting index at all.
CREATE INDEX IF NOT EXISTS idx_objects_folder_path_key
    ON objects(game_id, folder_path_key);

-- The dashboard's "recently added" list is ORDER BY indexed_at DESC LIMIT n,
-- which scanned and sorted the whole table on every dashboard open.
CREATE INDEX IF NOT EXISTS idx_mods_indexed_at
    ON mods(indexed_at DESC);

-- =========================================
-- MIGRATION: 20260815000000_normalize_mod_folder_paths.sql
-- =========================================

-- Normalize `mods.folder_path` to the mods-root-relative form.
--
-- Two writers disagreed: disk reconcile stored a path relative to the mods
-- root, the scan commit stored an absolute one. Readers had to guess, and
-- conflict detection guessed wrong -- it tested the column with a plain
-- existence check, which fails for a relative path and silently reported no
-- conflicts. The scan commit now writes the relative form; this brings rows
-- written before that in line.
--
-- Deliberately NOT touched:
--   * folder_path_key -- computed as root.join(path), so it is already the
--     same value for either form. Rewriting it would be a no-op at best.
--   * id -- derived from that key, and referenced by four tables
--     (collections, mod_versions, dedup whitelist pairs, and one SET NULL).
--     Nothing here needs the id to change.
--
-- Conservative by construction: a row is rewritten only when its path starts
-- with the game's mods root, case-insensitively, and the next character is a
-- separator. A trailing separator on mods_path, a mixed separator style, or a
-- path outside the root all fail that test and the row is left exactly as it
-- was. Skipping is always safe here; mangling a path is not.
UPDATE mods
SET folder_path = substr(mods.folder_path, length(g.mods_path) + 2)
FROM games g
WHERE g.id = mods.game_id
  AND g.mods_path IS NOT NULL
  AND g.mods_path <> ''
  AND length(mods.folder_path) > length(g.mods_path) + 1
  AND lower(substr(mods.folder_path, 1, length(g.mods_path))) = lower(g.mods_path)
  AND substr(mods.folder_path, length(g.mods_path) + 1, 1) IN ('/', '\');

-- =========================================
-- MIGRATION: 20260825000002_filesystem_identity.sql
-- =========================================

ALTER TABLE objects ADD COLUMN filesystem_identity TEXT;
ALTER TABLE mods ADD COLUMN filesystem_identity TEXT;

CREATE INDEX idx_objects_game_filesystem_identity
    ON objects(game_id, filesystem_identity)
    WHERE filesystem_identity IS NOT NULL;

CREATE INDEX idx_mods_game_filesystem_identity
    ON mods(game_id, filesystem_identity)
    WHERE filesystem_identity IS NOT NULL;

-- =========================================
-- MIGRATION: 20260828090000_normalize_legacy_object_categories.sql
-- =========================================

-- Normalize historical display categories into the canonical object taxonomy.
-- Child mods inherit their owning object's normalized category.

UPDATE objects
SET object_type = 'Weapon',
    sub_category = 'Light Cone'
WHERE object_type = 'Light Cone';

UPDATE objects
SET object_type = 'Weapon',
    sub_category = 'W-Engine'
WHERE object_type = 'W-Engine';

UPDATE objects
SET object_type = 'Character'
WHERE object_type = 'Resonator';

UPDATE objects
SET object_type = 'Other',
    sub_category = 'Echo'
WHERE object_type = 'Echo';

UPDATE objects
SET object_type = 'Other',
    sub_category = 'Bangboo'
WHERE object_type = 'Bangboo';

UPDATE objects
SET object_type = 'UI'
WHERE object_type IN ('User Interface', 'Interface');

UPDATE mods
SET object_type = (
    SELECT objects.object_type
    FROM objects
    WHERE objects.id = mods.object_id
)
WHERE object_id IS NOT NULL
  AND EXISTS (
      SELECT 1
      FROM objects
      WHERE objects.id = mods.object_id
        AND mods.object_type IS NOT objects.object_type
  );

-- =========================================
-- MIGRATION: 20260828091000_import_batches.sql
-- =========================================

ALTER TABLE games ADD COLUMN ready_to_move_path TEXT;

CREATE TABLE import_batches (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    flow TEXT NOT NULL CHECK(flow IN ('auto_import', 'specific_import', 'browser', 'ready_to_move')),
    target_mode TEXT NOT NULL CHECK(target_mode IN ('auto', 'specific')),
    target_object_id TEXT REFERENCES objects(id) ON DELETE SET NULL,
    target_subpath TEXT,
    source_archive_path TEXT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN (
        'draft', 'analyzing', 'awaiting_review', 'ready', 'committing',
        'partial', 'done', 'failed', 'cancelled'
    )),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT;

ALTER TABLE import_jobs ADD COLUMN batch_id TEXT REFERENCES import_batches(id) ON DELETE CASCADE;
ALTER TABLE import_jobs ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'browser_download';
ALTER TABLE import_jobs ADD COLUMN source_path TEXT;
ALTER TABLE import_jobs ADD COLUMN planned_name TEXT;
ALTER TABLE import_jobs ADD COLUMN match_sub_category TEXT;
ALTER TABLE import_jobs ADD COLUMN classification_metadata TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(classification_metadata));
ALTER TABLE import_jobs ADD COLUMN category_suggestions_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(category_suggestions_json));
ALTER TABLE import_jobs ADD COLUMN canonical_suggestions_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(canonical_suggestions_json));
ALTER TABLE import_jobs ADD COLUMN destination_suggestions_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(destination_suggestions_json));
ALTER TABLE import_jobs ADD COLUMN destination_object_id TEXT REFERENCES objects(id) ON DELETE SET NULL;
ALTER TABLE import_jobs ADD COLUMN destination_path TEXT;
ALTER TABLE import_jobs ADD COLUMN confidence_tier TEXT NOT NULL DEFAULT 'no_match';
ALTER TABLE import_jobs ADD COLUMN evidence_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(evidence_json));
ALTER TABLE import_jobs ADD COLUMN decision TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE import_jobs ADD COLUMN source_fingerprint TEXT CHECK(source_fingerprint IS NULL OR json_valid(source_fingerprint));
ALTER TABLE import_jobs ADD COLUMN source_inspection TEXT CHECK(source_inspection IS NULL OR json_valid(source_inspection));
ALTER TABLE import_jobs ADD COLUMN result TEXT;

INSERT INTO import_batches (
    id, game_id, flow, target_mode, status, created_at, updated_at
)
SELECT
    'legacy:' || id,
    game_id,
    'browser',
    'auto',
    CASE
        WHEN status = 'done' THEN 'done'
        WHEN status = 'canceled' THEN 'cancelled'
        WHEN status = 'failed' THEN 'failed'
        ELSE 'draft'
    END,
    COALESCE(created_at, CURRENT_TIMESTAMP),
    COALESCE(updated_at, CURRENT_TIMESTAMP)
FROM import_jobs
WHERE game_id IS NOT NULL;

UPDATE import_jobs
SET batch_id = 'legacy:' || id,
    source_path = archive_path,
    planned_name = archive_path,
    status = CASE
        WHEN status = 'done' THEN 'done'
        WHEN status = 'canceled' THEN 'cancelled'
        WHEN status = 'failed' THEN 'failed'
        ELSE 'discovered'
    END
WHERE game_id IS NOT NULL;

CREATE INDEX idx_import_batches_game_status
    ON import_batches(game_id, status, updated_at DESC);
CREATE INDEX idx_import_jobs_batch_status
    ON import_jobs(batch_id, status);

CREATE TRIGGER trg_import_batches_updated_at
AFTER UPDATE ON import_batches
FOR EACH ROW
BEGIN
    UPDATE import_batches SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- =========================================
-- MIGRATION: 20260829000000_mod_inbox.sql
-- =========================================

ALTER TABLE import_jobs ADD COLUMN source_group_id TEXT;
ALTER TABLE import_jobs ADD COLUMN processed_source_path TEXT;
ALTER TABLE import_jobs ADD COLUMN source_processed_at TEXT;
ALTER TABLE import_jobs ADD COLUMN source_deleted_at TEXT;
ALTER TABLE import_jobs ADD COLUMN destination_mod_id TEXT REFERENCES mods(id) ON DELETE SET NULL;

UPDATE import_jobs SET source_group_id = id WHERE source_group_id IS NULL;

CREATE INDEX idx_import_jobs_mod_inbox_history
    ON import_jobs(game_id, source_processed_at DESC)
    WHERE source_processed_at IS NOT NULL;

CREATE INDEX idx_import_jobs_source_group
    ON import_jobs(source_group_id);

