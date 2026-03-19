CREATE TABLE IF NOT EXISTS collection_object_states (
    collection_id TEXT NOT NULL,
    object_id TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT 1,
    PRIMARY KEY (collection_id, object_id),
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY (object_id) REFERENCES objects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_collection_object_states_collection_id
    ON collection_object_states(collection_id);

CREATE INDEX IF NOT EXISTS idx_collection_object_states_object_id
    ON collection_object_states(object_id);
