-- Fix duplicate objects: merge case-duplicate names (e.g. "hook" + "Hook")
-- Keep the row with richer data (has thumbnail_path), reassign linked mods.

-- Step 1: Reassign mods from duplicate objects to the primary per LOWER(name)
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

-- Step 2: Delete the duplicate object rows (keep only one per LOWER(name) per game)
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
