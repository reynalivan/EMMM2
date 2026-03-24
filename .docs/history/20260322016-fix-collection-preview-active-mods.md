# Fix Collection Preview Active Mods Display

## Context

The user discovered that when previewing the "Apply Collection" dialog, all mods (both enabled and disabled) belonging to the target objects were being displayed, rather than just the ones that were explicitly active/enabled in the target snapshot.

## Changes

- Modified `preview_apply` in `src-tauri/src/services/collection_service.rs` to query the `objects` and `mods` tables directly for `status = 'ENABLED'`, mirroring the exact logic used during `create_collection`.
- Modified `preview_switch_corridor` in `src-tauri/src/services/corridor_service.rs` to use the same logic for determining leaving members.
- Removed `enrich_objects_with_disk_mods` from both services. This function was breaking the DB snapshot paradigm by discarding DB mod records and re-scanning the entire disk folder on the fly, essentially injecting the current disk state (including disabled items) into collections instead of respecting the snapshot.
- Removed `discover_sub_mods` side-effect dead code.

## Impacted Files

- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)

## Goal

The Collection Preview modal front-end accurately reflects the actual snapshot saved in the Collection, filtering out inactive/disabled mods perfectly.

## Impact

- Cleaned up the "Apply Collection" preview to only show accurately enabled mods in both "Current" and "Target" collection panels.
- Switch Corridor modal preview has exactly the same accuracy improvements.
- Eliminated redundant `read_dir` file system calls during previews, strictly fetching the data from the local SQLite cache instead.
