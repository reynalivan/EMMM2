ALTER TABLE browser_downloads ADD COLUMN can_resume INTEGER NOT NULL DEFAULT 0;
ALTER TABLE browser_downloads ADD COLUMN tab_label TEXT;
