# Fix Broken `emmm://` Protocol Images

## Context

After implementing the secure custom protocol (`emmm://`) for serving thumbnail assets via Tauri's WebView, all images appeared broken. The root cause was that the backend returned raw custom scheme URIs which the frontend incorrectly wrapped using Tauri's `convertFileSrc()`. On Windows WebView2, custom protocols must be fetched via the `http://{scheme}.localhost/` format, not raw schemes.

## Changes

- Renamed the custom protocol from `emmm` to `emmm` in `src-tauri/src/lib.rs`.
- Updated `emmm` protocol handler in `lib.rs` to intercept `http://emmm.localhost/` and `https://emmm.localhost/` prefixes dynamically mapped by Tauri v2.
- Replaced all 5 occurrences in `thumbnail_cache.rs` and 1 in `mod_thumbnail_cmds.rs` to generate `http://emmm.localhost/game_id/path` URIs instead of raw `emmm://`.
- Removed `convertFileSrc()` double-wrapping from frontend thumbnail consumers (`FolderCard.tsx`, `FolderListRow.tsx`, `ObjectRowItem.tsx`, `CollectionModRow.tsx`).
- Updated `useThumbnail.test.tsx` to correctly mock `commands.getModThumbnail` after the Specta migration.
- Cleaned up 6 unused variable warnings and 1 missing `get_game_object_by_id` import in the backend (`organizer_ext.rs`, etc.).

## Impacted Files

### Backend (Rust)

- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/services/images/thumbnail_cache.rs` (modified)
- `src-tauri/src/commands/mods/mod_thumbnail_cmds.rs` (modified)
- `src-tauri/src/services/mods/organizer_ext.rs` (modified)
- `src-tauri/src/services/scanner/core/types.rs` (modified)
- `src-tauri/src/services/app/post_apply.rs` (modified)
- `src-tauri/src/services/mods/variant_service.rs` (modified)
- `src-tauri/src/commands/mods/mod_core_cmds.rs` (modified)

### Frontend (TypeScript/React)

- `src/features/folder-grid/FolderCard.tsx` (modified)
- `src/features/folder-grid/FolderListRow.tsx` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/features/collections/components/CollectionModRow.tsx` (modified)
- `src/hooks/useThumbnail.test.tsx` (modified)

## Goal

To properly and securely side-load validated image assets through the Tauri v2 `emmm://` custom protocol while preventing double-protocol mangling by the frontend.

## Impact

- All thumbnails across Folder Grid, Object List, and Collections are successfully restored.
- Resolved 25 hidden compilation errors and 7 warnings in the Rust backend test pipeline.
- Achieved a fully clean build (`cargo check`) and entirely passing frontend test suite.
