-- The init migration created browser_downloads with a schema that never
-- matched the service code (missing source_url/finished_at, wrong byte
-- column names), so every download insert failed at runtime. This table is
-- a transient download cache, so recreate it to match the code.
DROP TABLE IF EXISTS browser_downloads;

CREATE TABLE browser_downloads (
    id             TEXT PRIMARY KEY,
    session_id     TEXT,
    filename       TEXT NOT NULL,
    file_path      TEXT,
    source_url     TEXT,
    status         TEXT NOT NULL,
    bytes_received INTEGER NOT NULL DEFAULT 0,
    bytes_total    INTEGER,
    error_msg      TEXT,
    started_at     TEXT NOT NULL,
    finished_at    TEXT
) STRICT;

CREATE INDEX IF NOT EXISTS idx_browser_downloads_started_at
    ON browser_downloads (started_at DESC);
