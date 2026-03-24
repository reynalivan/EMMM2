# Greenfield V1 Backend & Frontend Cleanup

## Context

The Greenfield System Redesign introduced V2 modules (`domain/`, `repo/`, `pipeline/`, V2 services, V2 commands) but left V1 references in several backend and frontend files, causing compilation and type errors.

## Changes

### Backend — V1 Module References Removed

- **`lib.rs`**: Declared `pub mod domain`, `pub mod pipeline`, `pub mod repo`. Removed V1 startup hooks (`services::collections::materialize_game_collections_if_missing`, `services::corridor_runtime::reconcile_active_game_corridor`). Removed all `collection_cmds::*` command registrations. Replaced with V2 `v2_cmds::*` commands. Removed `set_safe_mode_enabled` and `preview_corridor_switch` from settings commands.
- **`services/mod.rs`**: Removed `corridor_types` module declaration.
- **`hotkeys/manager.rs`**: Switched from V1 `switch_mode_guarded` to V2 `corridor::switch_corridor`. Updated `list_collections` to V2 service. Fixed `applied_mod_count` → `mods_enabled` field access.
- **`sync_cmds.rs`**: Removed `reconcile_active_corridor_if_needed` function and invocations (stale `corridor_runtime` ref).
- **`mod_bulk_cmds.rs`**: Removed `bulk_toggle_mods_by_ids` command (stale `collection_repo` ref).

### Frontend — Dead V1 Files Removed

- Deleted `CollectionsPage.tsx` (658-line V1 version)
- Deleted `CollectionsPage.test.tsx` (V1 test suite)
- Deleted `ApplyCollectionModal.test.tsx` (test for deleted modal)
- Updated `index.ts` barrel: `CollectionsPage` now re-exports from `V2CollectionsPage`

## Impacted Files

- `src-tauri/src/lib.rs` (modified)
- `src-tauri/src/services/mod.rs` (modified)
- `src-tauri/src/services/hotkeys/manager.rs` (modified)
- `src-tauri/src/commands/scanner/sync_cmds.rs` (modified)
- `src-tauri/src/commands/mods/mod_bulk_cmds.rs` (modified)
- `src/features/collections/CollectionsPage.tsx` (deleted)
- `src/features/collections/CollectionsPage.test.tsx` (deleted)
- `src/features/collections/components/ApplyCollectionModal.test.tsx` (deleted)
- `src/features/collections/index.ts` (modified)

## Goal

Backend and frontend fully aligned with Greenfield V2 architecture. Zero V1 module references remain in active code paths.

## Impact

- `cargo check` → zero errors
- `cargo test` → zero failures
- `npx tsc --noEmit` → zero errors
- V1 collection/corridor/privacy commands no longer registered in Tauri
- Frontend barrel export now points to V2 page
