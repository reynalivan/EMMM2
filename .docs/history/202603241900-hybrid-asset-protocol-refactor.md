# Hybrid Asset Protocol Refactor

### Context

The custom `emmm://` protocol was causing reliability issues on Windows (host normalization artifacts) and added overhead. This refactor transitions the system to a Hybrid approach where the backend handles logic (resizing/caching) but returns absolute paths, which the frontend serves via Tauri's built-in `asset://` protocol.

### Changes

- **Backend (ThumbnailCache)**: Returns absolute WebP paths instead of `emmm://` URIs.
- **Backend (Scanner)**: `resolve_thumbnail` now returns absolute paths by joining with `mods_path` or `resource_dir`.
- **Backend (Core)**: Removed the ~90-line custom protocol registration from `lib.rs`.
- **Frontend (Hooks)**: `useThumbnail` now wraps backend results with `convertFileSrc()`.
- **Frontend (UI)**: Simplified `ObjectRowItem` logic; `getFileUrl` now prioritizes absolute path conversion.
- **Security**: Asset scope maintained at `["**"]` to support mod assets outside AppData (validated by PathGuard).

### Impacted Files

- `src-tauri/src/services/images/thumbnail_cache.rs` (modified)
- `src-tauri/src/commands/mods/mod_thumbnail_cmds.rs` (modified)
- `src-tauri/src/services/scanner/sync/preview.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/services/images/tests/thumbnail_cache_tests.rs` (modified)
- `src/hooks/useThumbnail.ts` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/lib/utils.ts` (modified)
- `src/hooks/useThumbnail.test.tsx` (modified)

### Goal

Standardize on Tauri's native `asset://` protocol for performance and stability while maintaining backend control over thumbnail generation.

### Impact

- Eliminates "Broken Image" placeholders caused by protocol normalization.
- Reduces backend codebase size by removing custom protocol logic.
- Maintains security via PathGuard validation.

### Notes

Backend unit tests reflect new absolute path returns. Frontend tests verify `convertFileSrc` integration.
