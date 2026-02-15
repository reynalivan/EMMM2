-- Epic 1: Add missing columns and constraints
ALTER TABLE collections ADD COLUMN is_safe_context BOOLEAN DEFAULT 0;
CREATE UNIQUE INDEX IF NOT EXISTS idx_games_path ON games(path);
