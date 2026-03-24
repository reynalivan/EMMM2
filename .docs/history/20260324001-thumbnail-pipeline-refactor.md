# Thumbnail Pipeline Refactor

## Context

Thumbnails were broken across ObjectList and FolderGrid. Root causes:

- `paste_thumbnail` returned an absolute FS path; stale cache was never invalidated.
- `getFileUrl` blindly passed all strings through `convertFileSrc`, which broke on `emmm://` URIs and relative Master DB paths.
- `GallerySection` hardcoded `convertFileSrc` directly.

## Changes

### Core Logic (before → after)

**`paste_thumbnail_inner`**

- Before: returned `target_path.to_string_lossy()` (absolute path)
- After: invalidates both image-keyed and folder-keyed cache, then calls `ThumbnailCache::get_thumbnail` and returns the resulting `http://emmm.localhost/thumbnails/{hash}.webp` URI

**`getFileUrl` (utils.ts)**

- Before: `convertFileSrc(path)` on all input strings
- After: passes `http://` / `https://` / `emmm://` URIs unchanged; uses `convertFileSrc` only for absolute FS paths; returns `''` for relative/unknown paths

**`GallerySection`**

- Before: `convertFileSrc(imagePath)` and `convertFileSrc(activePath)` directly
- After: `getFileUrl(imagePath)` and `getFileUrl(activePath)` — unified handler

## Impacted Files

- `src-tauri/src/commands/mods/mod_thumbnail_cmds.rs` (modified)
- `src/lib/utils.ts` (modified)
- `src/features/preview/components/GallerySection.tsx` (modified)

## Goal

All thumbnail rendering paths now produce valid browser-loadable URIs. Cache invalidation fires correctly on paste. No broken image placeholders from stale or unresolvable URIs.

## Impact

- `paste_thumbnail` now returns the cached WebP URI instead of a raw path — consumers using the return value (e.g., optimistic updates) now get a correct URL immediately.
- Relative Master DB `thumbnail_path` values in `ObjectRowItem` now gracefully return `''` (hide) instead of causing a broken `convertFileSrc` call.
- No breaking API changes to backend command signatures.
