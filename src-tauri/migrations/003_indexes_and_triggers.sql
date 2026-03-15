-- =============================================================================
-- INDEXES, TRIGGERS & CONSTRAINTS
-- All performance indexes, updated_at triggers, and uniqueness enforcement.
-- Must run after 001 and 002 as it references all tables.
-- Generated: 2026-03-15
-- =============================================================================

-- ---------------------------------------------------------------------------
-- MODS indexes
-- ---------------------------------------------------------------------------
CREATE UNIQUE INDEX IF NOT EXISTS idx_mods_folder_path
    ON mods(game_id, folder_path COLLATE NOCASE);

CREATE INDEX IF NOT EXISTS idx_mods_object_id   ON mods(object_id);
CREATE INDEX IF NOT EXISTS idx_mods_game_status ON mods(game_id, status);
CREATE INDEX IF NOT EXISTS idx_mods_pinned      ON mods(game_id, is_pinned);
CREATE INDEX IF NOT EXISTS idx_mods_favorite    ON mods(game_id, is_favorite);
CREATE INDEX IF NOT EXISTS idx_mods_updated_at  ON mods(updated_at);
CREATE INDEX IF NOT EXISTS idx_mods_indexed_at  ON mods(indexed_at);
CREATE INDEX IF NOT EXISTS idx_mods_object_type ON mods(object_type);
CREATE INDEX IF NOT EXISTS idx_mods_game_safe   ON mods(game_id, is_safe);

-- ---------------------------------------------------------------------------
-- OBJECTS indexes
-- ---------------------------------------------------------------------------
CREATE UNIQUE INDEX IF NOT EXISTS idx_objects_custom_folder
    ON objects(game_id, folder_path COLLATE NOCASE);

-- Case-insensitive unique name per game
CREATE UNIQUE INDEX IF NOT EXISTS idx_objects_game_name
    ON objects(game_id, name COLLATE NOCASE);

CREATE INDEX IF NOT EXISTS idx_objects_game_id     ON objects(game_id);
CREATE INDEX IF NOT EXISTS idx_objects_game_type   ON objects(game_id, object_type);
CREATE INDEX IF NOT EXISTS idx_objects_pinned      ON objects(game_id, is_pinned);
CREATE INDEX IF NOT EXISTS idx_objects_folder_path ON objects(game_id, folder_path);
CREATE INDEX IF NOT EXISTS idx_objects_updated_at  ON objects(updated_at);

-- Covering index for default ObjectList sort (is_pinned DESC, object_type, name)
CREATE INDEX IF NOT EXISTS idx_objects_default_sort
    ON objects(game_id, is_pinned DESC, object_type, name);

-- ---------------------------------------------------------------------------
-- COLLECTIONS indexes
-- ---------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_collections_game_context
    ON collections(game_id, is_safe_context);

CREATE UNIQUE INDEX IF NOT EXISTS idx_collections_game_name_context
    ON collections(game_id, name, is_safe_context);

CREATE INDEX IF NOT EXISTS idx_collections_unsaved   ON collections(game_id, is_last_unsaved);
CREATE INDEX IF NOT EXISTS idx_collections_updated_at ON collections(updated_at);

-- ---------------------------------------------------------------------------
-- COLLECTION ITEMS indexes
-- ---------------------------------------------------------------------------
CREATE UNIQUE INDEX IF NOT EXISTS idx_collection_items_unique
    ON collection_items(collection_id, mod_id);

CREATE INDEX IF NOT EXISTS idx_collection_items_collection_id ON collection_items(collection_id);
CREATE INDEX IF NOT EXISTS idx_collection_items_mod_id        ON collection_items(mod_id);
CREATE INDEX IF NOT EXISTS idx_collection_items_fallback      ON collection_items(collection_id, mod_path);

-- ---------------------------------------------------------------------------
-- SCAN RESULTS index
-- ---------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_scan_results_queue ON scan_results(game_id, status);

-- ---------------------------------------------------------------------------
-- DEDUP indexes
-- ---------------------------------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_dedup_jobs_game           ON dedup_jobs(game_id);
CREATE INDEX IF NOT EXISTS idx_dedup_groups_job          ON dedup_groups(job_id);
CREATE INDEX IF NOT EXISTS idx_dedup_groups_resolution_status ON dedup_groups(resolution_status);
CREATE INDEX IF NOT EXISTS idx_dedup_members_group       ON dedup_group_members(group_id);
CREATE INDEX IF NOT EXISTS idx_whitelist_game            ON duplicate_whitelist(game_id);

-- ---------------------------------------------------------------------------
-- updated_at TRIGGERS
-- Auto-update updated_at on any row modification.
-- ---------------------------------------------------------------------------

CREATE TRIGGER IF NOT EXISTS trg_mods_updated_at
AFTER UPDATE ON mods
FOR EACH ROW
BEGIN
    UPDATE mods SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_objects_updated_at
AFTER UPDATE ON objects
FOR EACH ROW
BEGIN
    UPDATE objects SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_collections_updated_at
AFTER UPDATE ON collections
FOR EACH ROW
BEGIN
    UPDATE collections SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- indexed_at: set on first INSERT only (never overwrite)
CREATE TRIGGER IF NOT EXISTS trg_mods_indexed_at
AFTER INSERT ON mods
FOR EACH ROW
WHEN NEW.indexed_at IS NULL
BEGIN
    UPDATE mods SET indexed_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
