# Canonical snapshot runtime and command access

## Context

- Frontend canonical flow already depended on runtime snapshot commands, but Tauri permission allowlist still missed them.
- Collections runtime also still depended on hybrid materialization tables, which kept active-state and preview logic more complex than needed.

## Changes

- Added missing Tauri allowlist entries for canonical collection runtime commands.
- Added canonical snapshot columns on `collections` and introduced `corridor_runtime_cache`.
- Moved collection runtime reads to `collections.snapshot_json`, `collections.signature`, and `collections.root_count`.
- Wrote corridor runtime snapshots into `corridor_runtime_cache` after strict runtime resolution.
- Kept legacy collection tables only as historical materialization input; runtime hot path no longer reads them first.
- Updated list/backfill flows and regression tests to validate canonical snapshot columns and runtime cache.

## Impacted Files

- `src-tauri/migrations/010_canonical_runtime_snapshots.sql` (added)
- `src-tauri/permissions/app-commands.toml` (modified)
- `src-tauri/src/database/mod.rs` (modified)
- `src-tauri/src/database/corridor_runtime_cache_repo.rs` (added)
- `src-tauri/src/database/collection_repo.rs` (modified)
- `src-tauri/src/database/settings_repo.rs` (modified)
- `src-tauri/src/services/collections/runtime_snapshot.rs` (modified)
- `src-tauri/src/services/collections/storage.rs` (modified)
- `src-tauri/src/services/collections/types.rs` (modified)
- `src-tauri/tests/collections_service.rs` (modified)

## Goal

- Canonical FE commands are allowed by Tauri permissions.
- Collection/runtime state now resolves from snapshot columns and runtime cache instead of join-heavy collection materialization tables.

## Impact

- Fixes `get_corridor_runtime_snapshot not allowed` once the app is rebuilt with the new permission manifest.
- Save/apply/runtime-read flows now share the same snapshot-based source of truth.
- Legacy tables are still present for historical backfill and old test fixtures, but they are no longer the primary runtime read path.

## Notes

- A rebuild is still required for the updated Tauri permission manifest to be embedded into the desktop app.
