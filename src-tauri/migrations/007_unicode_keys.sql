ALTER TABLE mods ADD COLUMN folder_path_key TEXT;
ALTER TABLE objects ADD COLUMN folder_path_key TEXT;
ALTER TABLE objects ADD COLUMN name_key TEXT;
ALTER TABLE collections ADD COLUMN name_key TEXT;
ALTER TABLE collection_items ADD COLUMN mod_path_key TEXT;
ALTER TABLE collection_nested_items ADD COLUMN mod_path_key TEXT;

DROP INDEX IF EXISTS idx_mods_folder_path;
DROP INDEX IF EXISTS idx_objects_custom_folder;
DROP INDEX IF EXISTS idx_objects_game_name;
DROP INDEX IF EXISTS idx_collections_game_name_context;

CREATE INDEX IF NOT EXISTS idx_mods_folder_path_key
    ON mods(game_id, folder_path_key);
CREATE INDEX IF NOT EXISTS idx_objects_folder_path_key
    ON objects(game_id, folder_path_key);
CREATE INDEX IF NOT EXISTS idx_objects_name_key
    ON objects(game_id, name_key);
CREATE INDEX IF NOT EXISTS idx_collections_name_key_context
    ON collections(game_id, name_key, is_safe_context);
CREATE INDEX IF NOT EXISTS idx_collection_items_mod_path_key
    ON collection_items(collection_id, mod_path_key);
CREATE INDEX IF NOT EXISTS idx_collection_nested_items_mod_path_key
    ON collection_nested_items(collection_id, mod_path_key);
