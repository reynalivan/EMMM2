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
    is_safe BOOLEAN NOT NULL DEFAULT 1,
    is_enabled BOOLEAN NOT NULL DEFAULT 1,
    thumbnail_hint TEXT,
    corridor_source TEXT,
    PRIMARY KEY (collection_id, root_path_key),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY (object_id) REFERENCES objects(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_collection_roots_collection_id
    ON collection_roots(collection_id);

CREATE INDEX IF NOT EXISTS idx_collection_roots_root_path_key
    ON collection_roots(root_path_key);

CREATE TABLE IF NOT EXISTS collection_signatures (
    collection_id TEXT PRIMARY KEY,
    signature TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_collection_signatures_signature
    ON collection_signatures(signature);
