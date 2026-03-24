# Mod Management Architecture Refactor

## Context

Unified the collision and sync logic to ensure end-to-end type safety and alignment between the database, backend, and filesystem. This addresses gaps in folder collision handling and conflict detection scope.

## Changes

- **Structured Collisions**: Added [CollisionInfo](file:///e:/Dev/EMMMNEW/src-tauri/src/services/scanner/core/types.rs) and [CollisionResolution](file:///e:/Dev/EMMMNEW/src-tauri/src/services/scanner/core/types.rs). Refactored [SyncResult](file:///e:/Dev/EMMMNEW/src-tauri/src/services/scanner/sync/types.rs) and [ExtractionResult](file:///e:/Dev/EMMMNEW/src-tauri/src/services/mods/archive/types.rs) to include a `collisions` field.
- **Conflict Scope**: Expanded [ConflictInfo](file:///e:/Dev/EMMMNEW/src-tauri/src/services/scanner/conflict/mod.rs) to include `ShaderOverride` and `Resource` hashes. Added `is_active` flag.
- **Type Safety**: Standardized on [AppError](file:///e:/Dev/EMMMNEW/src-tauri/src/domain/errors.rs) across command layers. Implemented `Clone` for [AppError](file:///e:/Dev/EMMMNEW/src-tauri/src/domain/errors.rs) to support bulk actions.
- **Service Integration**: Updated `mod_meta_cmds.rs` to pass `WatcherState` and `OperationLock` to the refactored [metadata](file:///e:/Dev/EMMMNEW/src-tauri/src/services/mods/metadata.rs) service.

## Impacted Files

- `src-tauri/src/services/scanner/core/types.rs` (modified)
- `src-tauri/src/services/scanner/conflict/mod.rs` (modified)
- `src-tauri/src/services/scanner/sync/types.rs` (modified)
- `src-tauri/src/services/mods/archive/types.rs` (modified)
- `src-tauri/src/commands/mods/mod_meta_cmds.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `src-tauri/src/domain/errors.rs` (modified)
- `src-tauri/src/services/app/post_apply.rs` (modified)

## Goal

A stable, type-safe mod management system that handles folder collisions non-blockingly and reports comprehensive conflicts.

## Impact

- **Security**: All mod metadata commands now use `PathGuard` and `OperationLock`.
- **Reliability**: Partial imports are now possible when folder collisions occur.
- **Type Safety**: End-to-end alignment with Specta for frontend bindings.
