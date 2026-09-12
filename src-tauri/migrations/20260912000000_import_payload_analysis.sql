-- Durable Mod Inbox analysis data. Existing batches remain readable and are
-- re-analyzed before commit because their revision remains zero.
ALTER TABLE import_jobs ADD COLUMN archive_sha256 TEXT;
ALTER TABLE import_jobs ADD COLUMN payload_manifest_json TEXT
    CHECK(payload_manifest_json IS NULL OR json_valid(payload_manifest_json));
ALTER TABLE import_jobs ADD COLUMN duplicate_of_item_id TEXT
    REFERENCES import_jobs(id) ON DELETE SET NULL;
ALTER TABLE import_jobs ADD COLUMN target_comparison_json TEXT
    CHECK(target_comparison_json IS NULL OR json_valid(target_comparison_json));
ALTER TABLE import_jobs ADD COLUMN analysis_revision INTEGER NOT NULL DEFAULT 0;
ALTER TABLE import_jobs ADD COLUMN analysis_ack_revision INTEGER;
ALTER TABLE import_jobs ADD COLUMN identity_match_status TEXT NOT NULL DEFAULT 'needs_review'
    CHECK(identity_match_status IN ('auto_matched', 'needs_review', 'no_match'));
ALTER TABLE import_jobs ADD COLUMN diagnostics_json TEXT NOT NULL DEFAULT '[]'
    CHECK(json_valid(diagnostics_json));
ALTER TABLE import_jobs ADD COLUMN content_kind TEXT NOT NULL DEFAULT 'unknown'
    CHECK(content_kind IN ('skin', 'patch', 'utility', 'foreign_game', 'unknown'));
ALTER TABLE import_jobs ADD COLUMN package_shape TEXT NOT NULL DEFAULT 'single'
    CHECK(package_shape IN ('single', 'composite', 'bundle'));
ALTER TABLE import_jobs ADD COLUMN source_order INTEGER NOT NULL DEFAULT 0;
ALTER TABLE import_jobs ADD COLUMN root_order INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_import_jobs_archive_sha256
    ON import_jobs(archive_sha256)
    WHERE archive_sha256 IS NOT NULL;
CREATE INDEX idx_import_jobs_duplicate_of_item_id
    ON import_jobs(duplicate_of_item_id)
    WHERE duplicate_of_item_id IS NOT NULL;
