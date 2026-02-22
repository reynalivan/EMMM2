-- Epic 9: Fix Duplicate Objects strict enforcement
-- 1. Run the merge logic again just in case the backend recreated duplicates (e.g., via repair_orphan_mods) before this migration applied.
UPDATE mods SET object_id = (
    SELECT o_primary.id
    FROM objects o_primary
    WHERE o_primary.game_id = (SELECT game_id FROM objects WHERE id = mods.object_id)
      AND LOWER(o_primary.name) = (SELECT LOWER(name) FROM objects WHERE id = mods.object_id)
    ORDER BY
        CASE WHEN o_primary.thumbnail_path IS NOT NULL THEN 0 ELSE 1 END,
        o_primary.created_at ASC
    LIMIT 1
)
WHERE object_id IN (
    SELECT dup.id FROM objects dup
    WHERE EXISTS (
        SELECT 1 FROM objects other
        WHERE other.game_id = dup.game_id
          AND LOWER(other.name) = LOWER(dup.name)
          AND other.id <> dup.id
    )
);

DELETE FROM objects WHERE id NOT IN (
    SELECT id FROM (
        SELECT id,
               ROW_NUMBER() OVER (
                   PARTITION BY game_id, LOWER(name)
                   ORDER BY
                       CASE WHEN thumbnail_path IS NOT NULL THEN 0 ELSE 1 END,
                       created_at ASC
               ) AS rn
        FROM objects
    ) ranked
    WHERE rn = 1
);

-- 2. Drop the original case-sensitive unique index
DROP INDEX IF EXISTS idx_objects_game_name;

-- 3. Create a strict case-insensitive unique index
CREATE UNIQUE INDEX idx_objects_game_name ON objects(game_id, name COLLATE NOCASE);
