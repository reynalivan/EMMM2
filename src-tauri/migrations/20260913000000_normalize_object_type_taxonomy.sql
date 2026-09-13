-- Keep persisted categories compatible with the four-value taxonomy used by
-- object filters and the randomizer. Unknown values remain untouched so they
-- can be reviewed rather than silently reclassified.
UPDATE objects
SET object_type = CASE lower(trim(object_type))
    WHEN 'char' THEN 'Character'
    WHEN 'character' THEN 'Character'
    WHEN 'weapon' THEN 'Weapon'
    WHEN 'ui' THEN 'UI'
    WHEN 'other' THEN 'Other'
    ELSE object_type
END
WHERE lower(trim(object_type)) IN ('char', 'character', 'weapon', 'ui', 'other');

-- `mods.object_type` is a denormalized display/filter field. Restore it from
-- the Object source of truth for every associated mod.
UPDATE mods
SET object_type = (
    SELECT objects.object_type
    FROM objects
    WHERE objects.id = mods.object_id
)
WHERE object_id IS NOT NULL
  AND EXISTS (
      SELECT 1
      FROM objects
      WHERE objects.id = mods.object_id
        AND mods.object_type IS NOT objects.object_type
  );
