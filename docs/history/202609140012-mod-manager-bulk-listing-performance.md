# Bulk metadata and listing performance

## Context

Large bulk edits repeated `info.json` reads, issued one favorite/pin SQL update per mod, and repeatedly derived listing metadata during one workspace response.

## Changes

- Bulk metadata now parses the bytes already retained for rollback and writes one final document for missing metadata. File work and rollback run in the blocking worker pool.
- Favorite and pin updates normalize, deduplicate, and update path keys in bounded transaction batches.
- Workspace enrichment stores owner and safety indexes once, then root and active listings reuse them. Bulk toggle planning scans each distinct parent directory once.
- Grid bulk callbacks depend on stable mutation functions and the active game ID rather than React Query result wrappers.

## Impacted Files

- `src-tauri/src/modules/library/application/mods/{info_json.rs,bulk/attributes.rs,bulk/toggle.rs,core_ops/naming.rs,core_ops/toggle.rs}` (modified)
- `src-tauri/src/modules/library/adapters/sqlite/mods/batch.rs` (modified)
- `src-tauri/src/modules/workspace/application/{explorer/listing/owners.rs,workspace/mod.rs}` (modified)
- `src-tauri/src/modules/library/application/mods/tests/{info_json_tests.rs,bulk_attributes_tests.rs}` (modified)
- `src/widgets/mod-explorer/hooks/{useFolderGridBulk.ts,useFolderGridBulk.test.ts}` (modified/added)

## Goal

Bulk metadata and Mods Manager navigation do less repeated I/O and lookup work while filesystem state remains authoritative and reconcile remains the database projection path.

## Notes

Frontend typecheck, lint, and the focused callback-stability test passed. Targeted Rust tests are currently blocked before compilation by stale local SQLx schema metadata for `games.mods_path` and `ready_to_move_path`.
