-- A completed duplicate report is only valid for the exact Mods root scanned.
-- Existing reports lack this proof and are intentionally hidden until rescanned.
ALTER TABLE dedup_jobs ADD COLUMN root_path TEXT;
