-- Add folder_path to objects to represent the physical folder on disk.
-- The existing 'name' column now serves as the logical display alias.
ALTER TABLE objects ADD COLUMN folder_path TEXT;

-- Backfill legacy objects: they were previously forced to match their physical folder names.
UPDATE objects SET folder_path = name WHERE folder_path IS NULL;

-- Index for fast physical folder to object DB lookups during tree scanning
CREATE INDEX IF NOT EXISTS idx_objects_folder_path ON objects(game_id, folder_path);
