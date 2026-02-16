-- Migration 007: Drop unused trash_log table
-- The trash system uses filesystem-based metadata.json exclusively.
-- This table was created in migration 005 but never used by any Rust code.

DROP TABLE IF EXISTS trash_log;
DROP INDEX IF EXISTS idx_trash_game;
