# EMMM Architectural Stabilization & SQL Consistency

## Context

Architectural gaps were identified in the mod management system, specifically regarding identity persistence across toggles, SQL type mismatches for status columns, and missing task recovery logic. These gaps threatened the atomicity and reliability of the mod switching pipeline.

## Changes

- **Identity Stabilization**: Updated `collection_service.rs` and `corridor_service.rs` to use `mod_id` (BLAKE3 hash) for signature calculation instead of the volatile `mod_path`.
- **SQL Type Standardisation**: Migrated all remaining string-based status comparisons (`'ENABLED'`, `'DISABLED'`) in SQL queries to integer-based literals (`1`, `0`) to match the `ItemStatus` enum (i64) and DB schema.
- **Schema Alignment**: Corrected `init.sql` to align the `tasks` table with the `task_repo.rs` expectations (adding `game_id`, `task_type`, `target_id`, and `updated_at`).
- **Resilience**:
  - Implemented `RETRY` logic in `resolve_recovery_task` to allow re-running interrupted or crashed `apply_collection` and `switch_corridor` tasks.
  - Added a 7-day automatic garbage collection for the `tasks` table on startup.
  - Fixed a `SYSTEM` reason fallback in corridor switches to restore previously system-disabled mods when no user collection is present.

## Impacted Files

- `src-tauri/migrations/20260323000000_init.sql` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/services/startup_sync.rs` (modified)
- `src-tauri/src/pipeline/steps/batch_db_update.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `src-tauri/src/repo/task_repo.rs` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)

## Goal

Achieve a fully synchronized, type-safe, and resilient mod management architecture where database, backend logic, and filesystem naming remain consistent and recoverable even after crashes or unexpected interruptions.

## Impact

- Stable "Dirty" state tracking for mod collections.
- Correct boot-time and switch-time mod restoration.
- Type-consistent SQL operations preventing silent failures.
- Robust task recovery path for backend bulk operations.
