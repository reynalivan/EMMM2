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
