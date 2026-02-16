-- Epic 9: Duplicate scanner persistence and reporting tables

CREATE TABLE IF NOT EXISTS dedup_jobs (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    status TEXT NOT NULL DEFAULT 'in_progress' CHECK (status IN ('in_progress', 'completed', 'cancelled')),
    scanned_mods_count INTEGER NOT NULL DEFAULT 0,
    duplicate_groups_count INTEGER NOT NULL DEFAULT 0,
    duplicate_members_count INTEGER NOT NULL DEFAULT 0,
    stats_json JSON NOT NULL DEFAULT '{}',
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS dedup_groups (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL,
    confidence INTEGER NOT NULL DEFAULT 0 CHECK (confidence >= 0 AND confidence <= 100),
    primary_signal TEXT NOT NULL,
    match_reasons_json JSON NOT NULL DEFAULT '[]',
    resolution_status TEXT NOT NULL DEFAULT 'pending' CHECK (resolution_status IN ('pending', 'resolved', 'ignored', 'partial')),
    resolved_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (job_id) REFERENCES dedup_jobs(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS dedup_group_members (
    id TEXT PRIMARY KEY,
    group_id TEXT NOT NULL,
    folder_id TEXT NOT NULL,
    file_hash TEXT,
    signals_json JSON NOT NULL DEFAULT '{}',
    is_primary BOOLEAN NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (group_id) REFERENCES dedup_groups(id) ON DELETE CASCADE,
    FOREIGN KEY (folder_id) REFERENCES mods(id) ON DELETE CASCADE,
    UNIQUE (group_id, folder_id)
);

CREATE TABLE IF NOT EXISTS duplicate_whitelist (
    id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    folder_a_id TEXT NOT NULL,
    folder_b_id TEXT NOT NULL,
    ignored_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reason TEXT,
    FOREIGN KEY (game_id) REFERENCES games(id) ON DELETE CASCADE,
    FOREIGN KEY (folder_a_id) REFERENCES mods(id) ON DELETE CASCADE,
    FOREIGN KEY (folder_b_id) REFERENCES mods(id) ON DELETE CASCADE,
    CHECK (folder_a_id <> folder_b_id),
    CHECK (folder_a_id < folder_b_id),
    UNIQUE (game_id, folder_a_id, folder_b_id)
);

CREATE INDEX IF NOT EXISTS idx_dedup_jobs_game ON dedup_jobs(game_id);
CREATE INDEX IF NOT EXISTS idx_dedup_groups_job ON dedup_groups(job_id);
CREATE INDEX IF NOT EXISTS idx_dedup_groups_resolution_status ON dedup_groups(resolution_status);
CREATE INDEX IF NOT EXISTS idx_dedup_members_group ON dedup_group_members(group_id);
CREATE INDEX IF NOT EXISTS idx_whitelist_game ON duplicate_whitelist(game_id);
