-- Epic 12: Key-value store for metadata sync state
CREATE TABLE IF NOT EXISTS app_meta (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

-- Seed initial sync state
INSERT OR IGNORE INTO app_meta (key, value) VALUES ('metadata_version', '0');
INSERT OR IGNORE INTO app_meta (key, value) VALUES ('last_modified', '');
INSERT OR IGNORE INTO app_meta (key, value) VALUES ('etag', '');
