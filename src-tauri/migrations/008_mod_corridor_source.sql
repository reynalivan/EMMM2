ALTER TABLE mods ADD COLUMN corridor_source TEXT NOT NULL DEFAULT 'unknown';

UPDATE mods
SET corridor_source = 'unknown'
WHERE corridor_source IS NULL OR TRIM(corridor_source) = '';

CREATE INDEX IF NOT EXISTS idx_mods_game_status_safe_source
    ON mods(game_id, status, is_safe, corridor_source);
