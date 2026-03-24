# Batch 2: Boot Sequence, Recovery Dialog & ObjectList Payload

## Context

Implemented the application crash recovery cycle for the corridor switch/apply pipelines, ensuring the app handles interrupted migrations safely upon reboot. Also enriched the `ObjectList` payload (`active_mod_paths`) required to highlight active mods accurately.

## Changes

- Updated `ObjectSummary` to include aggregated `active_mod_paths_joined`.
- Implemented `app_startup_check` & `clear_pending_tasks` for fetching and managing interrupted DB pipeline tasks.
- Introduced a full-screen `RecoveryDialog` blocking `App.tsx` routes if `app_startup_check` detects pending tasks, preventing corrupt app states.
- Enhanced backend to expose a global task query `get_all_pending_tasks_global`.

## Impacted Files

- `src-tauri/src/database/object_repo.rs` (modified query)
- `src-tauri/src/commands/collections/cmds.rs` (added app_startup_check, clear_pending_tasks)
- `src-tauri/src/repo/task_repo.rs` (added get_all_pending_tasks_global)
- `src-tauri/src/lib.rs` (registered new commands)
- `src-tauri/permissions/app-commands.toml` (added permissions)
- `src/types/object.ts` (added active_mod_paths_joined field)
- `src/types/task.ts` (new types)
- `src/App.tsx` (wrapped router with recovery boot sequence logic)
- `src/features/collections/components/RecoveryDialog.tsx` (new component)

## Goal

The system now gracefully handles pipeline interruptions (crash recovery) right at boot and accurately exposes `active_mod_paths` per object.

## Impact

- Failsafe crash recovery mechanism blocks UI until acknowledged by user.
- The `ObjectList` can now display what modifications are active accurately.
