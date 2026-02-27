-- Epic 14: Database Schema Hardening

-- 1. Add unique index with COLLATE NOCASE for physical folder paths
CREATE UNIQUE INDEX IF NOT EXISTS idx_mods_folder_path ON mods(game_id, folder_path COLLATE NOCASE);
CREATE UNIQUE INDEX IF NOT EXISTS idx_objects_custom_folder ON objects(game_id, folder_path COLLATE NOCASE);

-- 2. Add updated_at columns
ALTER TABLE mods ADD COLUMN updated_at DATETIME DEFAULT CURRENT_TIMESTAMP;
ALTER TABLE objects ADD COLUMN updated_at DATETIME DEFAULT CURRENT_TIMESTAMP;
ALTER TABLE collections ADD COLUMN updated_at DATETIME DEFAULT CURRENT_TIMESTAMP;

-- Create AFTER UPDATE triggers to auto-update the timestamp
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

-- Add basic indices on updated_at to support future delta-sync querying
CREATE INDEX IF NOT EXISTS idx_mods_updated_at ON mods(updated_at);
CREATE INDEX IF NOT EXISTS idx_objects_updated_at ON objects(updated_at);
CREATE INDEX IF NOT EXISTS idx_collections_updated_at ON collections(updated_at);

-- 3. Data Integrity Checks (where possible with SQLite ALTER)
-- Note: Cannot ALTER TABLE ADD CHECK directly without table rebuild. 
-- For scan_results status, we'll recreate the table if we must, or just enforce in Rust.
-- Actually, SQLite 3.35.0+ allows ALTER TABLE ADD COLUMN with CHECK, but not adding CHECK to existing columns.
-- We will rely on Rust strict models for this.

-- 4. Add missing performance indices
CREATE INDEX IF NOT EXISTS idx_scan_results_queue ON scan_results(game_id, status);
CREATE INDEX IF NOT EXISTS idx_collection_items_fallback ON collection_items(collection_id, mod_path);
