# Gap Fix: Mod Ops Security & DB Sync (Epic 13/14/20/21)

## Context

Post Tauri-Specta migration, bulk ops lacked path validation (S-1/S-2 security gaps), rename missed dirty-state sync (D-3), child path DB failures were silently swallowed (D-1/D-2), and FolderContentInfo struct didn't match req-21 spec. FE cache missed conflict invalidation on rename/delete.

## Changes

### Priority 1 — Security

- `mod_bulk_cmds.rs`: Added `game_id: String` to `bulk_toggle_mods`; fetches `mods_path` from DB; validates ALL paths via `is_path_safe` before lock acquisition. Same for `bulk_delete_mods`. Removed orphaned `bulk_toggle_mods_inner`/`bulk_delete_mods_inner` helpers.
- `bulk.rs::bulk_toggle`: Added `mods_path: &str` and `game_id: &str` params. Absolute paths from `toggle_mod_inner` now stripped to relative before `batch_update_path_and_status`.

### Priority 2 — DB Sync Integrity

- `bulk.rs::bulk_toggle`: Adds `handle_mod_moved_or_renamed` per-item success for collection auto-healing. Adds `corridor_service::recompute_signature` (both safe/unsafe) after batch completion. Adds done progress event.
- `core_ops.rs::toggle_mod_inner_service`: `let _` on `update_child_paths` → `log::warn!`.
- `core_ops.rs::rename_mod_folder_inner_service`: `let _` on `update_mod_path_by_old_path_in_game` → `log::warn!`. Child path updates → `log::warn!`. Added `handle_dirty_state` call at end of rename so corridor dirty detection stays accurate.

### Priority 3 — Spec Compliance

- `mod_core_cmds.rs::FolderContentInfo`: Added `ini_count`, `image_count`, `nested_folder_count`, `total_size_bytes` fields per req-21 AC-21.2.1 spec.
- `check_folder_contents`: Replaced single-level `read_dir` count with bounded DFS walker (max 1000 entries per AC-21.2.3). Counts `.ini`, image files, nested folders, total bytes.

### Priority 5 — FE Cache

- `useFolders.ts::useRenameMod`: Added `['conflicts']` cache invalidation on success.
- `useFolders.ts::useDeleteMod`: Added `['conflicts']` cache invalidation on success.

## Impacted Files

- `src-tauri/src/commands/mods/mod_bulk_cmds.rs` (modified — rewritten)
- `src-tauri/src/services/mods/bulk.rs` (modified — rewritten)
- `src-tauri/src/services/mods/core_ops.rs` (modified)
- `src-tauri/src/commands/mods/mod_core_cmds.rs` (modified)
- `src/hooks/useFolders.ts` (modified)

## Goal

Bulk operations are now safe: paths validated against mods_path before processing. Rename updates dirty state. DB failures produce warnings instead of silent data loss. FolderContentInfo matches the req-21 spec. Conflict cache stays fresh after rename/delete.

## Impact

- `bulk_toggle_mods` command signature changed: `game_id` is now required. Bindings will regenerate on next `pnpm tauri dev`.
- `bulk_toggle` service signature changed: callers now pass `mods_path` and `game_id`.
- `FolderContentInfo` type expanded: FE bindings will reflect new fields after regeneration. `item_count` still present for backward compat.

## Open Gaps (Not Fixed This Session)

- BulkActionBar for FolderGrid (AC-14.1.2) — not implemented in FolderGrid.tsx; only ObjectList has it.
- `bulk_ops.rs::bulk_toggle_mods` — parallel+atomic implementation still orphaned (not wired to any command). Marked for future promotion.
- Windows 260-char path limit validation (AC-21.1.6) — deferred.
