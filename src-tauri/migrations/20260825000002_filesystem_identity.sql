ALTER TABLE objects ADD COLUMN filesystem_identity TEXT;
ALTER TABLE mods ADD COLUMN filesystem_identity TEXT;

CREATE INDEX idx_objects_game_filesystem_identity
    ON objects(game_id, filesystem_identity)
    WHERE filesystem_identity IS NOT NULL;

CREATE INDEX idx_mods_game_filesystem_identity
    ON mods(game_id, filesystem_identity)
    WHERE filesystem_identity IS NOT NULL;
