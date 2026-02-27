-- Epic 7: Add Privacy Mode tracking colums
ALTER TABLE mods ADD COLUMN last_status_sfw BOOLEAN;
ALTER TABLE mods ADD COLUMN last_status_nsfw BOOLEAN;
