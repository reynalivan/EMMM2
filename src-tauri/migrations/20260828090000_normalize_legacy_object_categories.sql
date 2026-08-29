-- Normalize historical display categories into the canonical object taxonomy.
-- Child mods inherit their owning object's normalized category.

UPDATE objects
SET object_type = 'Weapon',
    sub_category = 'Light Cone'
WHERE object_type = 'Light Cone';

UPDATE objects
SET object_type = 'Weapon',
    sub_category = 'W-Engine'
WHERE object_type = 'W-Engine';

UPDATE objects
SET object_type = 'Character'
WHERE object_type = 'Resonator';

UPDATE objects
SET object_type = 'Other',
    sub_category = 'Echo'
WHERE object_type = 'Echo';

UPDATE objects
SET object_type = 'Other',
    sub_category = 'Bangboo'
WHERE object_type = 'Bangboo';

UPDATE objects
SET object_type = 'UI'
WHERE object_type IN ('User Interface', 'Interface');

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
