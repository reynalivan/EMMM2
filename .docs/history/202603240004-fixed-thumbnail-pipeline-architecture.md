# 202603240004-fixed-thumbnail-pipeline-architecture

## Context

Thumbnail images were not appearing in the mod management system because of architectural mismatches between the custom `emmm://` protocol handler, the command bindings, and the frontend consumption layer.

## Changes

- **Backend (lib.rs)**: Added a cache-directory bypass to the `emmm` protocol handler to serve generated WebP thumbnails without triggering `PathGuard` 403 blocks.
- **Bindings (bindings.ts)**: Corrected `getModThumbnail` to invoke the proper `'get_mod_thumbnail'` Rust command instead of `'get_thumbnail'`.
- **Frontend Hook (useThumbnail.ts)**: Updated to accept and propagate `game_id` to the backend.
- **Frontend Consumers**: Updated `FolderCard.tsx`, `FolderListRow.tsx`, `ObjectRowItem.tsx`, and `CollectionModRow.tsx` to pass the active `game_id` to the thumbnail resolver.
- **Cleanup**: Removed stale `convertFileSrc` imports in grid/list views.

## Impacted Files

- `src-tauri/src/lib.rs` (modified)
- `src/lib/bindings.ts` (modified)
- `src/hooks/useThumbnail.ts` (modified)
- `src/features/folder-grid/FolderCard.tsx` (modified)
- `src/features/folder-grid/FolderListRow.tsx` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/features/collections/components/CollectionModRow.tsx` (modified)
- `src/hooks/useThumbnail.test.tsx` (modified)
- `src/features/folder-grid/FolderListRow.test.tsx` (modified)

## Goal

Restore end-to-end thumbnail rendering across all mod management views (Grid, List, and Collections).

## Impact

Thumbnails now correctly resolve from the backend cache and are served via the `http://emmm.localhost/` protocol. Performance is maintained through L1/L2 caching and concurrency limits.
