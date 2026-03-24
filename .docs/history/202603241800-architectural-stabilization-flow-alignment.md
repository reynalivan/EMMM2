# Architectural Stabilization: Mod Management Flow Alignment

## Context

Refactored the mod management and discovery pipelines to resolve architectural gaps, enforce strict type safety (integer-based status), and ensure consistent Safe Mode categorization across manual, background (watcher), and silent (startup) indexing flows.

## Changes

- **Security**: Injected `OperationLock` into `switch_corridor`, `apply_collection`, and `undo_collection` to serialize filesystem mutations.
- **Type Safety**: Shifted `BatchDbUpdate` pipeline from string-based status ('ENABLED'/'DISABLED') to integer-based (1/0) for schema compliance.
- **Discovery**:
  - Centralized keyword-based safety classification in `scanner::sync::helpers::classify_corridor`.
  - Fixed watcher and startup sync logic to discover and tag mods correctly using `Safe Mode` keywords.
  - Modified `mod_repo::insert_new_mod` to accept dynamic safety and source flags.
- **Recovery**: Standardized task recovery payloads in `switch_pipeline.rs` and `apply_pipeline.rs`.

## Impacted Files

- `src-tauri/src/repo/mod_repo.rs` (modified)
- `src-tauri/src/services/scanner/sync/helpers.rs` (modified)
- `src-tauri/src/services/scanner/sync/commit.rs` (modified)
- `src-tauri/src/services/scanner/watcher/lifecycle.rs` (modified)
- `src-tauri/src/services/scanner/object_sync.rs` (modified)
- `src-tauri/src/commands/collections/cmds.rs` (modified)
- `src-tauri/src/pipeline/steps/batch_db_update.rs` (modified)

## Goal

Achieve a unified, type-safe, and secure mod management architecture that aligns with the system flow documentation and prevents data-filesystem drift.

## Result

The system now correctly categorizes discovered mods (Safe/Unsafe) across all entry points and ensures atomic, locked filesystem operations for corridor switches.
