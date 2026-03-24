# Batch 1 Core Engine Impl: Virtual Collections, Path Healing & Instrumentation

## Context

Implementing core application functionality required for EMMM robustly. The key goals included cross-collection auto-healing when paths mutate on disk, gracefully handling "unsaved" loaded states, pre-flight disk validation for mods, preventing incomplete operations from causing corruption via crash instrumentation, and frontend integration for missing mods.

## Changes

- **Pre-Apply Disk Validation:** Instead of trying to operate on vanished disk paths, pipelines now proactively validate `target_members` matching disk structure. If paths are missing, an explicitly structured `MISSING_MODS` error handles asking the user via dialog.
- **Dirty State Tracking:** Manual toggles trigger `handle_dirty_state` creating an `unsaved` collection snapshot automatically.
- **Auto-Healing `source_mod_id`:** Renaming/moving mods safely propagates updates across all saved collections referencing the previous exact structure using `update_member_paths`.
- **Active Snapshot on Delete:** Deleting the active collection now creates an `unsaved` snapshot copy first, protecting current loadouts.
- **`SwitchResult` Updates:** Corridor switching now forwards tracking metadata (`restored_collection_id`) directly to the frontend context.
- **Pipeline Task Tracking:** Created a persistent SQLite `tasks` table with PENDING/COMPLETED crash protection for apply/switch corridor sequences.

## Impacted Files

### Backend (Rust)

- `src-tauri/migrations/014_unsaved_kind_index.sql` (added)
- `src-tauri/migrations/015_collection_member_source_mod.sql` (added)
- `src-tauri/migrations/016_pipeline_tasks.sql` (added)
- `src-tauri/src/domain/collection.rs`, `src-tauri/src/domain/corridor.rs`, `src-tauri/src/domain/errors.rs`, `src-tauri/src/domain/task.rs` (modified/added)
- `src-tauri/src/repo/collection_repo.rs`, `src-tauri/src/repo/task_repo.rs` (modified/added)
- `src-tauri/src/services/collection_service.rs`, `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/pipeline/apply_pipeline.rs`, `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src-tauri/src/lib.rs`, `src-tauri/permissions/app-commands.toml` (modified)

### Frontend (React)

- `src/features/collections/hooks/useCollections.ts` (modified)
- `src/features/collections/components/ApplyCollectionModal.tsx` (modified)

## Goal

The database correctly synchronizes and mirrors reality under severe constraints like random missing files on disk and physical path renaming. State machine guarantees protection against unexpected process terminations.

## Impact

- **Side effects:** Mod renaming successfully heals old pointers universally. Pre-apply validations prevent corruption from out-of-date snapshots.
- **Performance:** Fast UUID `source_mod_id` indexing introduced.
- **Breaking changes:** `apply_collection` frontend mutation/backend service now accept an `ignore_missing` bypass boolean context.
