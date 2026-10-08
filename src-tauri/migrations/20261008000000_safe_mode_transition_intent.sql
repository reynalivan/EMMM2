CREATE TABLE safe_mode_snapshots (
    game_id TEXT PRIMARY KEY NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    snapshot_json TEXT NOT NULL
);

CREATE TABLE safe_mode_task_intents (
    task_id TEXT PRIMARY KEY NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    previous_enabled INTEGER NOT NULL CHECK (previous_enabled IN (0, 1)),
    target_enabled INTEGER NOT NULL CHECK (target_enabled IN (0, 1)),
    target_snapshot_json TEXT NOT NULL,
    rollback_snapshot_json TEXT NOT NULL,
    previous_restore_json TEXT,
    rollback_requested INTEGER NOT NULL DEFAULT 0 CHECK (rollback_requested IN (0, 1))
);
