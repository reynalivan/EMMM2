# Corridor Switch Pipeline & Dirty State Detection Fix

## Context

User reported 4 functional bugs during Greenfield end-to-end verification:

1. Corridor switch didn't remember the last active collection.
2. Collection switch/save broken due to missing actual FS renames when switching corridors.
3. Mod toggle didn't mark the corridor as dirty ("unsaved").
4. Preview switcher didn't show target mod changes.

Investigation revealed `switch_pipeline.rs` was a stub (Phase 1 placeholders) that only ran SQL `UPDATE mods SET status`, missing FS renames, undo snapshot creation, and signature computation.

## Changes

- **`switch_pipeline.rs`**: Completely rewritten.
  - Snapshot step now uses `collection_repo::create(UndoSnapshot)` to save leaving state.
  - Disable/Restore steps now use `bulk_ops::bulk_toggle_mods` to perform proper concurrent FS folder renames alongside DB updates.
  - Update step recomputes and stores the corridor's BLAKE3 signature.
- **`corridor_service.rs`**: Updated `switch_corridor` signature to inject `&WatcherState` (needed for `bulk_toggle_mods`).
- **`corridor_service.rs`**: Fixed `preview_switch` target member resolution to gracefully fall back from `undo_collection_id` to `active_collection_id` to `SYSTEM` disabled queries.
- **`core_ops.rs`**: Added `recompute_signature` execution at the end of `toggle_mod_inner_service` so manual mod toggling triggers dirty state.
- **`cmds.rs` & `manager.rs`**: Wired `watcher_state` down to the `switch_corridor` call.

## Impacted Files

- `src-tauri/src/pipeline/switch_pipeline.rs` (modified heavily)
- `src-tauri/src/services/mods/core_ops.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src-tauri/src/services/hotkeys/manager.rs` (modified)

## Goal

To properly rename files on disk during privacy/corridor mode switching, and reliably track corridor signatures/dirty-states when mods are manually toggled or switched.

## Impact

- Corridor switches now accurately rename folders with the `DISABLED` prefix and fully execute transactional DB updates.
- Undo snapshots are properly generated, creating functional "history" between safe mode transitions.
- Preview switcher cleanly delegates differences.
- High impact on the filesystem operations logic, but reuses battle-tested `bulk_ops` eliminating unvetted risk.

## Notes

- Discovered `WatcherState` is un-Cloneable (contains a `Mutex<Watcher>`). Remedied by passing it purely as a reference into the pipeline functions.
