-- Staging table for scan results (transient, cleared after confirm)
CREATE TABLE IF NOT EXISTS scan_results (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    source_path TEXT NOT NULL,
    source_name TEXT NOT NULL,
    matched_object TEXT,
    object_type TEXT,
    match_level TEXT,
    match_confidence TEXT,
    match_detail TEXT,
    detected_skin TEXT,
    thumbnail_path TEXT,
    status TEXT DEFAULT 'PENDING',
    error_message TEXT,
    FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
);
