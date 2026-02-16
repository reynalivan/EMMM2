-- Epic 8: Collections index and uniqueness alignment
-- Canonical tables: collections, collection_items

CREATE UNIQUE INDEX IF NOT EXISTS idx_collection_items_unique
ON collection_items(collection_id, mod_id);

CREATE INDEX IF NOT EXISTS idx_collection_items_collection_id
ON collection_items(collection_id);

CREATE INDEX IF NOT EXISTS idx_collection_items_mod_id
ON collection_items(mod_id);

CREATE INDEX IF NOT EXISTS idx_collections_game_context
ON collections(game_id, is_safe_context);

CREATE UNIQUE INDEX IF NOT EXISTS idx_collections_game_name_context
ON collections(game_id, name, is_safe_context);
