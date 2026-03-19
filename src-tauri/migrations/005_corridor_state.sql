-- Up: Track which named collection is "active" in each corridor,
--      plus the undo snapshot pointer.
--
-- This replaces the fragile frontend synthesis of 'virtual-unsaved' entries
-- and allows the UI to know the canonical current-state collection.
CREATE TABLE IF NOT EXISTS corridor_state (
    game_id                 TEXT NOT NULL,
    is_safe                 INTEGER NOT NULL CHECK(is_safe IN (0, 1)),
    -- Last named collection applied to this corridor (NULL = unsaved / never applied)
    active_collection_id    TEXT REFERENCES collections(id) ON DELETE SET NULL,
    -- Points to the is_last_unsaved snapshot used for Undo (NULL = no undo available)
    undo_collection_id      TEXT REFERENCES collections(id) ON DELETE SET NULL,
    PRIMARY KEY (game_id, is_safe)
);
