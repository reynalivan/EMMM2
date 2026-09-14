-- Cached local catalog-identity checks. A row is kept for both a suggested
-- identity and a completed check with no suggestion so reconciliation does
-- not make the dashboard repeatedly inspect unchanged objects.
CREATE TABLE object_identity_checks (
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    object_id TEXT NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    object_revision TEXT NOT NULL,
    source_fingerprint TEXT NOT NULL CHECK(json_valid(source_fingerprint)),
    catalog_id TEXT NOT NULL,
    catalog_version TEXT NOT NULL,
    matcher_revision INTEGER NOT NULL,
    candidate_entry_key TEXT,
    candidate_name TEXT,
    confidence_percentage INTEGER,
    match_status TEXT,
    evidence_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(evidence_json)),
    thumbnail_path TEXT,
    checked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (game_id, object_id)
) STRICT;

CREATE INDEX object_identity_checks_review_idx
    ON object_identity_checks (game_id, catalog_id, catalog_version, candidate_entry_key);

-- Dismissals are deliberately tied to both the catalog source and candidate
-- identity. Updating a community pack cannot resurrect an item the user has
-- explicitly rejected for the same published identity.
CREATE TABLE object_identity_dismissals (
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    object_id TEXT NOT NULL REFERENCES objects(id) ON DELETE CASCADE,
    catalog_id TEXT NOT NULL,
    candidate_entry_key TEXT NOT NULL,
    dismissed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (game_id, object_id, catalog_id, candidate_entry_key)
) STRICT;

-- Packs installed before provenance existed are intentionally represented by
-- no row and read as Local. The singleton key enforces the one-active-pack
-- product contract.
CREATE TABLE catalog_pack_provenance (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    source_kind TEXT NOT NULL CHECK(source_kind IN ('local', 'trusted', 'community')),
    source_url TEXT,
    repository TEXT,
    release_tag TEXT,
    asset_name TEXT,
    asset_digest TEXT,
    installed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT;
