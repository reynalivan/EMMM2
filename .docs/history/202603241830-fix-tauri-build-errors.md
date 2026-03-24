# Fix Tauri Backend Build Errors

## Context

Resolve compilation errors following architectural refactoring of OperationLock, CommandResult, and Repository layers.

## Changes

- **OperationLock**: Changed `.lock()` to `.acquire()` in `src/commands/collections/cmds.rs`.
- **sync_objects_for_game**: Added missing `mods_path` argument in `object_cmds.rs` and `startup_sync.rs`.
- **delete_mod_by_path**: Removed extra `game_id` argument in `collision_resolver.rs`.
- **tauri::Manager**: Imported trait in `lifecycle.rs` for `try_state` support.
- **CommandError**: Fixed type mismatches where `AppError` was passed instead of `String`.
- **Cleanups**: Removed unused imports and prefixed unused variables.

## Impacted Files

- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src-tauri/src/commands/objects/object_cmds.rs` (modified)
- `src-tauri/src/services/startup_sync.rs` (modified)
- `src-tauri/src/services/mods/collision_resolver.rs` (modified)
- `src-tauri/src/services/scanner/watcher/lifecycle.rs` (modified)
- `src-tauri/src/services/scanner/object_sync.rs` (modified)
- `src-tauri/src/services/scanner/sync/commit.rs` (modified)
- `src-tauri/src/services/mods/bulk.rs` (modified)

## Goal

Stable, buildable backend with correct API usage and type safety.

## Impact

- Restored core mod management functionality and synchronization.
- Resolved all identified compilation blockers.
