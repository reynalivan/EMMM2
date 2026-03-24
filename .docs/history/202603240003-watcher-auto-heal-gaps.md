# Watcher Depth Rename Auto-Healing

## Context

A gap analysis against `flow.md` revealed that external object and mod renaming (Depth 1 and Depth 2) detected by the file watcher correctly updated basic database records but failed completely to update linked collection signatures or mod paths. This left collections perpetually broken whenever users reorganized mods manually via Windows Explorer.

## Changes

- Replaced direct, limited DB path mutation in `lifecycle.rs` with newly implemented, transaction-aware `handle_object_renamed_tx` and `handle_mod_moved_or_renamed_tx` services.
- Wired Depth 1 external renames to iterate through and surgically replace affected prefix paths inside all assigned `collection_mods`, followed by automatic signature recomputations.
- Removed dead code (`resolve_dest` in `extract.rs`) and unused variables (`app` in `conflict_cmds.rs`) to ensure a warning-free compilation.

## Impacted Files

- `src-tauri/src/repo/collection_repo.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/scanner/watcher/lifecycle.rs` (modified)
- `src-tauri/src/commands/scanner/conflict_cmds.rs` (modified)
- `src-tauri/src/services/mods/archive/extract.rs` (modified)

## Goal

The system now faithfully adheres to Section 6 of `flow.md`. External mod reorganization is seamlessly auto-healed in real time, preventing loadout corruption.

## Impact

- **Side effects**: A robust cascade of DB updates correctly reflects physical reality without blocking.
- **Performance**: Safe collection healing prevents hashing loops via transaction batching.
