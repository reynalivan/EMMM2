-- User-installed catalog packs replace bundled character databases and images.
-- Preserve manually chosen local thumbnails; only remove paths known to be from
-- the former bundled catalog resource tree.
ALTER TABLE objects ADD COLUMN thumbnail_source TEXT
    CHECK (thumbnail_source IS NULL OR thumbnail_source IN ('user', 'asset_pack'));

UPDATE objects
SET thumbnail_path = NULL,
    thumbnail_source = NULL
WHERE thumbnail_path IS NOT NULL
  AND lower(replace(thumbnail_path, '\\', '/')) LIKE '%/databases/thumbnails/%';
