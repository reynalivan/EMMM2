-- Gap 6: Add mod_path fallback column for resilience when mod IDs change
ALTER TABLE collection_items ADD COLUMN mod_path TEXT;
