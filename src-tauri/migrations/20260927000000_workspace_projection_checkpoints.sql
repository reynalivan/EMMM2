CREATE TABLE IF NOT EXISTS workspace_projection_checkpoints (
    game_id TEXT NOT NULL,
    source_epoch TEXT NOT NULL,
    projected_revision INTEGER NOT NULL CHECK (projected_revision >= 0),
    PRIMARY KEY (game_id, source_epoch)
);
