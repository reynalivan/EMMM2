# Modernize Database Schema & Split-Table Collection Architecture

## Context

Refactored the application's core data layer to align with the "Ultimate Baseline" schema, migrate to a high-performance split-table design for collections, and remove 100% of legacy shims.

## Changes

- **Database Schema**: Updated `init.sql` to include `object_type`, `sub_category`, and `status` in `objects`. Enforced SQLite `STRICT` mode with `INTEGER` booleans.
- **Split-Table Migration**: Fully adopted `collection_mods`, `collection_objects`, and `collection_roots` across all layers.
- **Repository Alignment**: Restored all deleted utility functions in `object_repo.rs` and aligned `collection_repo.rs` with the new schema.
- **Service & Pipeline Refactor**: Updated `corridor_service.rs`, `collection_service.rs`, and the `ApplyPipeline` to eliminate `CollectionMember` usage.
- **Type Safety**: Renamed `is_safe_context` to `is_safe` throughout the backend and added missing `FromRow` implementations.

## Impacted Files

- `migrations/20260323000000_init.sql` (modified)
- `src-tauri/src/database/object_repo.rs` (modified)
- `src-tauri/src/repo/collection_repo.rs` (modified)
- `src-tauri/src/repo/corridor_repo.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)
- `src-tauri/src/pipeline/apply_pipeline.rs` (modified)
- `src-tauri/src/pipeline/steps/*.rs` (modified)
- `src-tauri/src/domain/collection.rs` (modified)
- `src-tauri/src/domain/errors.rs` (modified)
- `src-tauri/src/services/objects/mutate.rs` (modified)
- `src-tauri/src/services/objects/query.rs` (modified)

## Goal

Establish a robust, future-proof database baseline and a performant collection system that strictly follows filesystem truth.

## Impact

- **Performance**: Faster collection queries due to normalized split tables.
- **Stability**: Elimination of legacy shims reduces maintenance overhead and potential bugs.
- **Integrity**: SQLite `STRICT` mode ensures data type consistency.
