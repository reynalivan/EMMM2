ALTER TABLE games ADD COLUMN launch_mode TEXT NOT NULL DEFAULT 'standalone';
ALTER TABLE games ADD COLUMN xxmi_launcher_exe TEXT;
