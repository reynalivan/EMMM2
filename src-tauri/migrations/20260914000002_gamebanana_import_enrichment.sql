-- Preserve a verified GameBanana page reference with a Discover download so
-- import analysis can enrich it without inferring identity from a filename.
ALTER TABLE browser_downloads ADD COLUMN origin_page_url TEXT;
ALTER TABLE browser_downloads ADD COLUMN gamebanana_item_type TEXT;
ALTER TABLE browser_downloads ADD COLUMN gamebanana_item_id INTEGER;
ALTER TABLE browser_downloads ADD COLUMN gamebanana_content_sha256 TEXT;

CREATE INDEX idx_browser_downloads_gamebanana_item
    ON browser_downloads (game_id, gamebanana_item_type, gamebanana_item_id, started_at DESC);

CREATE INDEX idx_browser_downloads_gamebanana_source
    ON browser_downloads (game_id, file_path COLLATE NOCASE, status, bytes_received, started_at DESC);

-- Source metadata is distinct from user/category classification metadata. It
-- stores optional remote provenance and never drives a filesystem mutation.
ALTER TABLE import_jobs ADD COLUMN source_metadata_json TEXT NOT NULL DEFAULT '{}'
    CHECK(json_valid(source_metadata_json));
