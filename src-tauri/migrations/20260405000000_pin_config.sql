-- ------------------------------------------------------------------------------
-- PIN security singleton config
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS pin_config (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    pin_hash TEXT,
    recovery_hash TEXT,
    failed_attempts INTEGER NOT NULL DEFAULT 0,
    lockout_until TEXT,
    updated_at TEXT DEFAULT CURRENT_TIMESTAMP
) STRICT;

INSERT OR IGNORE INTO pin_config (id, failed_attempts, updated_at)
VALUES (1, 0, CURRENT_TIMESTAMP);
