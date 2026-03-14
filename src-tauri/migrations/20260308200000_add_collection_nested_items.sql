CREATE TABLE IF NOT EXISTS collection_nested_items (
    collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    mod_path TEXT NOT NULL,
    PRIMARY KEY (collection_id, mod_path)
);
