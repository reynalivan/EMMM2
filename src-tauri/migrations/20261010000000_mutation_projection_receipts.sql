CREATE TABLE mutation_projection_receipts (
    operation_id TEXT PRIMARY KEY,
    game_id TEXT NOT NULL,
    source_epoch TEXT NOT NULL
);
