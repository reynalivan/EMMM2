# Runtime Side-Effects Collections KeyViewer Alignment

## Context

Runtime mutations still mixed ownership for collection dirty-state and Dynamic KeyViewer refresh. Disk Reconcile already handled watcher/internal file mutations, but explicit toggle/rename/delete/move flows still triggered side-effects ad-hoc and could drift or double-fire.

## Changes

- Added shared backend helper to finalize runtime side-effects for collections dirty-state and KeyViewer refresh.
- Routed Disk Reconcile finalize step through the shared helper.
- Moved explicit runtime mutation flows to the shared helper:
  - single mod toggle
  - mod rename
  - move mod to object
  - toggle mod safe
  - object root toggle
  - bulk toggle
  - bulk delete
  - enable-only-this conflict resolution
  - delete to trash
- Removed duplicate command-layer overlay refresh calls where the service is now the owner.
- Added conflict-resolution runtime sync for renamed duplicate paths:
  - heal `mods.folder_path`
  - heal saved collection mod paths
  - rebuild runtime projection
- Added explicit trigger matrices to runtime flow, collections, and keyviewer docs.

## Impacted Files

- `.docs/flow.md` (modified)
- `.docs/requirements/req-31-collections.md` (modified)
- `.docs/requirements/req-43-dynamic-keyviewer.md` (modified)
- `src-tauri/src/services/app/mod.rs` (modified)
- `src-tauri/src/services/app/post_apply.rs` (modified)
- `src-tauri/src/services/app/runtime_effects.rs` (added)
- `src-tauri/src/services/disk_reconcile/orchestrator.rs` (modified)
- `src-tauri/src/services/mods/core_ops.rs` (modified)
- `src-tauri/src/services/mods/metadata.rs` (modified)
- `src-tauri/src/services/mods/organizer_ext.rs` (modified)
- `src-tauri/src/services/mods/object_switch.rs` (modified)
- `src-tauri/src/services/mods/trash.rs` (modified)
- `src-tauri/src/services/mods/bulk.rs` (modified)
- `src-tauri/src/services/scanner/conflict/mod.rs` (modified)
- `src-tauri/src/commands/mods/mod_bulk_cmds.rs` (modified)
- `src-tauri/src/commands/mods/mod_core_cmds.rs` (modified)
- `src-tauri/src/commands/mods/mod_meta_cmds.rs` (modified)
- `src-tauri/src/commands/mods/trash_cmds.rs` (modified)
- `src-tauri/src/commands/mods/conflict_cmds.rs` (modified)
- `src-tauri/src/commands/app/workspace_cmds.rs` (modified)

## Goal

Runtime changes now have one clear side-effect contract:
- Disk Reconcile owns watcher/external/internal-mutation refresh
- explicit runtime mutation services own their own collection dirty-state and KeyViewer refresh

## Impact

- Active collections now stay dirty/unsaved consistently after relevant explicit runtime mutations.
- KeyViewer artifacts refresh consistently after explicit runtime mutations without duplicate command-layer triggers.
- Conflict resolution now keeps collection/runtime paths in sync instead of stopping at a raw folder rename.
- Restore-from-trash remains watcher/Disk Reconcile owned because it is not watcher-suppressed.

## Notes

- Settings and hotkey overlay refresh commands were intentionally left as-is because they are not runtime mod mutation flows.
