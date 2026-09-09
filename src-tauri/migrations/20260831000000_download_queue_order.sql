-- Persist the scheduler's admission order so queued rows remain deterministic
-- even when several confirmations share the same second-level timestamp.
ALTER TABLE browser_downloads
    ADD COLUMN queue_order INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_browser_downloads_queue_order
    ON browser_downloads (status, queue_order);
