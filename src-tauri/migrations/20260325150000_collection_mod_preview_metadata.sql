ALTER TABLE collection_mods ADD COLUMN preview_path TEXT;

ALTER TABLE collection_mods ADD COLUMN node_type TEXT;

ALTER TABLE collection_mods
ADD COLUMN warnings_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(warnings_json));
