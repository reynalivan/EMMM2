-- Catalog packs are metadata-only. Preserve user thumbnails while removing
-- paths that pointed at artwork distributed by older catalog packs.
UPDATE objects
SET thumbnail_path = NULL,
    thumbnail_source = NULL
WHERE thumbnail_source = 'asset_pack';
