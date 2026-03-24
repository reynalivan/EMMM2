# Fix Cargo Check Compilation Errors

## Context

After DB schema refactor (TEXT→INTEGER for `game_type`/`status`, new `hash_db`/`custom_skins` columns), the backend had numerous compilation errors from type mismatches, missing trait impls, and stale error handling patterns.

## Changes

- **Error Handling Unification**: `CorridorError::Db` and `CollectionError::Db` changed from `sqlx::Error` to `String`. All `map_err` call sites updated to use `e.to_string()` or `?` with `From<sqlx::Error>` impls.
- **`ItemStatus` enum**: Added `FromStr`, `from_is_disabled`, `as_str`. Removed redundant `AppError` from `database/models.rs`.
- **`GameType` consistency**: `GameConfig.game_type` is now `GameType` enum. Fixed `game_cmds.rs` to parse `&str` → `GameType` and use the enum directly.
- **`specta::Type` derivations**: Added to `ArchiveInfo`, `RenameResult`, `PinStatus` for Specta command compatibility.
- **`CorridorMismatch` fields**: Changed from `&'static str` → `String` in `domain/errors.rs`.
- **New metadata fields**: Added `hash_db_json`, `custom_skins_json`, `db_thumbnail` to `ScanPreviewItem` and `ConfirmedScanItem`.
- **Nexus removal**: Removed `browser_fetch_nexus_info` from `lib.rs` command registration.
- **Unused imports/vars**: Cleaned up across `object_repo.rs`, `dashboard.rs`, `commit.rs`.

## Impacted Files

- `src-tauri/src/database/models.rs` (modified)
- `src-tauri/src/database/object_repo.rs` (modified)
- `src-tauri/src/domain/errors.rs` (modified)
- `src-tauri/src/domain/pin.rs` (modified)
- `src-tauri/src/commands/app/game_cmds.rs` (modified)
- `src-tauri/src/commands/scanner/sync_cmds.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/pipeline/apply_pipeline.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `src-tauri/src/pipeline/steps/batch_db_update.rs` (modified)
- `src-tauri/src/pipeline/steps/resolve_current_state.rs` (modified)
- `src-tauri/src/pipeline/steps/snapshot_state.rs` (modified)
- `src-tauri/src/pipeline/steps/update_corridor.rs` (modified)
- `src-tauri/src/pipeline/steps/validate_corridor.rs` (modified)
- `src-tauri/src/repo/task_repo.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/services/config/models.rs` (modified)
- `src-tauri/src/services/mods/core_ops.rs` (modified)
- `src-tauri/src/services/mods/metadata.rs` (modified)
- `src-tauri/src/services/app/dashboard.rs` (modified)
- `src-tauri/src/services/scanner/core/walker.rs` (modified)
- `src-tauri/src/services/scanner/sync/commit.rs` (modified)
- `src-tauri/src/services/scanner/sync/preview.rs` (modified)
- `src-tauri/src/services/scanner/sync/types.rs` (modified)
- `src-tauri/src/services/scanner/object_sync.rs` (modified)
- `src-tauri/src/services/scanner/watcher/lifecycle.rs` (modified)

## Goal

Backend compiles cleanly with `cargo check` — 0 errors, 0 warnings.

## Impact

- All error types now use `String` internally (Serde/Specta safe).
- Frontend bindings will need regeneration (`bindings.ts`) to pick up new enum types and struct changes.
- Frontend lint errors in `useMasterDbSync.ts` and `usePreviewData.ts` still need attention.
