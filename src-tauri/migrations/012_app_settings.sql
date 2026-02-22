-- Key-Value settings table for app-wide configuration
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Add columns to games table to match ConfigService's GameConfig fields
-- mod_path: absolute path to /Mods folder
-- game_exe: absolute path to the game executable
-- loader_exe: optional path to a 3DMigoto loader executable
ALTER TABLE games ADD COLUMN mod_path TEXT;
ALTER TABLE games ADD COLUMN game_exe TEXT;
ALTER TABLE games ADD COLUMN loader_exe TEXT;
