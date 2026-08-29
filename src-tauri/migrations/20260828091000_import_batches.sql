ALTER TABLE games ADD COLUMN ready_to_move_path TEXT;

CREATE TABLE import_batches (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    flow TEXT NOT NULL CHECK(flow IN ('auto_import', 'specific_import', 'browser', 'ready_to_move')),
    target_mode TEXT NOT NULL CHECK(target_mode IN ('auto', 'specific')),
    target_object_id TEXT REFERENCES objects(id) ON DELETE SET NULL,
    target_subpath TEXT,
    source_archive_path TEXT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK(status IN (
        'draft', 'analyzing', 'awaiting_review', 'ready', 'committing',
        'partial', 'done', 'failed', 'cancelled'
    )),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT;

ALTER TABLE import_jobs ADD COLUMN batch_id TEXT REFERENCES import_batches(id) ON DELETE CASCADE;
ALTER TABLE import_jobs ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'browser_download';
ALTER TABLE import_jobs ADD COLUMN source_path TEXT;
ALTER TABLE import_jobs ADD COLUMN planned_name TEXT;
ALTER TABLE import_jobs ADD COLUMN match_sub_category TEXT;
ALTER TABLE import_jobs ADD COLUMN classification_metadata TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(classification_metadata));
ALTER TABLE import_jobs ADD COLUMN category_suggestions_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(category_suggestions_json));
ALTER TABLE import_jobs ADD COLUMN canonical_suggestions_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(canonical_suggestions_json));
ALTER TABLE import_jobs ADD COLUMN destination_suggestions_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(destination_suggestions_json));
ALTER TABLE import_jobs ADD COLUMN destination_object_id TEXT REFERENCES objects(id) ON DELETE SET NULL;
ALTER TABLE import_jobs ADD COLUMN destination_path TEXT;
ALTER TABLE import_jobs ADD COLUMN confidence_tier TEXT NOT NULL DEFAULT 'no_match';
ALTER TABLE import_jobs ADD COLUMN evidence_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(evidence_json));
ALTER TABLE import_jobs ADD COLUMN decision TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE import_jobs ADD COLUMN source_fingerprint TEXT CHECK(source_fingerprint IS NULL OR json_valid(source_fingerprint));
ALTER TABLE import_jobs ADD COLUMN source_inspection TEXT CHECK(source_inspection IS NULL OR json_valid(source_inspection));
ALTER TABLE import_jobs ADD COLUMN result TEXT;

INSERT INTO import_batches (
    id, game_id, flow, target_mode, status, created_at, updated_at
)
SELECT
    'legacy:' || id,
    game_id,
    'browser',
    'auto',
    CASE
        WHEN status = 'done' THEN 'done'
        WHEN status = 'canceled' THEN 'cancelled'
        WHEN status = 'failed' THEN 'failed'
        ELSE 'draft'
    END,
    COALESCE(created_at, CURRENT_TIMESTAMP),
    COALESCE(updated_at, CURRENT_TIMESTAMP)
FROM import_jobs
WHERE game_id IS NOT NULL;

UPDATE import_jobs
SET batch_id = 'legacy:' || id,
    source_path = archive_path,
    planned_name = archive_path,
    status = CASE
        WHEN status = 'done' THEN 'done'
        WHEN status = 'canceled' THEN 'cancelled'
        WHEN status = 'failed' THEN 'failed'
        ELSE 'discovered'
    END
WHERE game_id IS NOT NULL;

CREATE INDEX idx_import_batches_game_status
    ON import_batches(game_id, status, updated_at DESC);
CREATE INDEX idx_import_jobs_batch_status
    ON import_jobs(batch_id, status);

CREATE TRIGGER trg_import_batches_updated_at
AFTER UPDATE ON import_batches
FOR EACH ROW
BEGIN
    UPDATE import_batches SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
