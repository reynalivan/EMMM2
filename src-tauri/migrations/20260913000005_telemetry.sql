-- Durable, privacy-preserving telemetry queue.  These tables intentionally
-- contain only bounded enum labels and hashes; raw errors, paths, and user
-- identifiers must never be written here.

CREATE TABLE telemetry_rollups (
    day_utc TEXT NOT NULL,
    release TEXT NOT NULL CHECK(length(release) <= 64),
    operation TEXT NOT NULL CHECK(operation IN (
        'onboarding', 'classification', 'classification_review', 'auto_match',
        'watcher', 'reconcile', 'toggle', 'bulk_action', 'import', 'extract',
        'collection_apply', 'restore', 'launch', 'error'
    )),
    outcome TEXT NOT NULL CHECK(outcome IN (
        'started', 'success', 'partial', 'failed', 'auto_accepted',
        'needs_review', 'matched', 'unmatched', 'accepted', 'rejected',
        'cancelled', 'triggered', 'overflow'
    )),
    error_code TEXT NOT NULL CHECK(error_code IN (
        'none', 'unknown', 'validation', 'io', 'database', 'network',
        'conflict', 'timeout', 'cancelled', 'panic', 'invariant', 'permission',
        'not_found', 'unsupported', 'external'
    )),
    count INTEGER NOT NULL CHECK(count > 0),
    duration_ms_total INTEGER NOT NULL DEFAULT 0 CHECK(duration_ms_total >= 0),
    duration_sample_count INTEGER NOT NULL DEFAULT 0 CHECK(duration_sample_count >= 0),
    exported_count INTEGER NOT NULL DEFAULT 0 CHECK(exported_count >= 0),
    exported_duration_ms_total INTEGER NOT NULL DEFAULT 0 CHECK(exported_duration_ms_total >= 0),
    exported_duration_sample_count INTEGER NOT NULL DEFAULT 0 CHECK(exported_duration_sample_count >= 0),
    CHECK(exported_count <= count),
    CHECK(exported_duration_ms_total <= duration_ms_total),
    CHECK(exported_duration_sample_count <= duration_sample_count),
    PRIMARY KEY (day_utc, release, operation, outcome, error_code)
) STRICT;

CREATE TABLE error_fingerprints (
    release TEXT NOT NULL CHECK(length(release) <= 64),
    fingerprint TEXT NOT NULL CHECK(length(fingerprint) = 64),
    operation TEXT NOT NULL CHECK(operation IN (
        'onboarding', 'classification', 'classification_review', 'auto_match',
        'watcher', 'reconcile', 'toggle', 'bulk_action', 'import', 'extract',
        'collection_apply', 'restore', 'launch', 'error'
    )),
    error_code TEXT NOT NULL CHECK(error_code IN (
        'none', 'unknown', 'validation', 'io', 'database', 'network',
        'conflict', 'timeout', 'cancelled', 'panic', 'invariant', 'permission',
        'not_found', 'unsupported', 'external'
    )),
    first_seen_at_utc TEXT NOT NULL,
    last_seen_at_utc TEXT NOT NULL,
    occurrence_count INTEGER NOT NULL DEFAULT 1 CHECK(occurrence_count > 0),
    reported_at_utc TEXT,
    PRIMARY KEY (release, fingerprint)
) STRICT;

-- A release may retain at most 100 first-seen error fingerprints. Existing
-- fingerprints still update their counters after the cap is reached.
CREATE TRIGGER telemetry_limit_fingerprints_per_release
BEFORE INSERT ON error_fingerprints
FOR EACH ROW
WHEN (
    SELECT COUNT(*)
    FROM error_fingerprints
    WHERE release = NEW.release
) >= 100
BEGIN
    SELECT RAISE(IGNORE);
END;

CREATE INDEX idx_error_fingerprints_pending_export
    ON error_fingerprints(reported_at_utc, first_seen_at_utc);

CREATE TABLE pending_crash_report (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    release TEXT NOT NULL CHECK(length(release) <= 64),
    operation TEXT NOT NULL CHECK(operation IN (
        'onboarding', 'classification', 'classification_review', 'auto_match',
        'watcher', 'reconcile', 'toggle', 'bulk_action', 'import', 'extract',
        'collection_apply', 'restore', 'launch', 'error'
    )),
    error_code TEXT NOT NULL CHECK(error_code IN (
        'none', 'unknown', 'validation', 'io', 'database', 'network',
        'conflict', 'timeout', 'cancelled', 'panic', 'invariant', 'permission',
        'not_found', 'unsupported', 'external'
    )),
    fingerprint TEXT NOT NULL CHECK(length(fingerprint) = 64),
    source TEXT NOT NULL CHECK(source IN ('rust_panic', 'previous_session', 'webview')),
    created_at_utc TEXT NOT NULL
) STRICT;

CREATE TABLE telemetry_export_state (
    stream TEXT PRIMARY KEY CHECK(stream = 'rollups'),
    last_attempt_at_utc TEXT,
    last_success_at_utc TEXT,
    last_status TEXT NOT NULL DEFAULT 'never' CHECK(last_status IN ('never', 'success', 'failed'))
) STRICT;
