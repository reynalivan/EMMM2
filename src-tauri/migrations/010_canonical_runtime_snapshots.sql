ALTER TABLE collections ADD COLUMN snapshot_json TEXT;
ALTER TABLE collections ADD COLUMN signature TEXT;
ALTER TABLE collections ADD COLUMN root_count INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_collections_signature_lookup
    ON collections(game_id, is_safe_context, is_last_unsaved, signature);

CREATE TABLE IF NOT EXISTS corridor_runtime_cache (
    game_id TEXT NOT NULL,
    is_safe INTEGER NOT NULL CHECK(is_safe IN (0, 1)),
    matched_collection_id TEXT REFERENCES collections(id) ON DELETE SET NULL,
    state_kind TEXT NOT NULL,
    state_name TEXT,
    signature TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    snapshot_source TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (game_id, is_safe)
);

CREATE INDEX IF NOT EXISTS idx_corridor_runtime_cache_match
    ON corridor_runtime_cache(matched_collection_id);
