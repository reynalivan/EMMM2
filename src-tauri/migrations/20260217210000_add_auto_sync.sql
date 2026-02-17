-- Add auto_sync column to objects table
ALTER TABLE objects ADD COLUMN auto_sync BOOLEAN NOT NULL DEFAULT 0;
