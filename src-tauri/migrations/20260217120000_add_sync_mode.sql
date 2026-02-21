-- Add is_auto_sync column to objects table
ALTER TABLE objects ADD COLUMN is_auto_sync BOOLEAN NOT NULL DEFAULT 0;
