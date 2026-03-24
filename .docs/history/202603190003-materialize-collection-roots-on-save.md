# Materialize Collection Roots on Save

## Context

- Real app data showed `collection_roots` and `collection_signatures` staying empty after save/apply flows.
- Empty runtime materialization caused strict active matching to fall back to `Unsaved Preset`.

## Changes

- Save-current collections now persist `collection_roots` and `collection_signatures` directly from the live corridor runtime snapshot.
- Unsaved snapshots now persist runtime materialization explicitly at snapshot creation time instead of relying on later lazy backfill.
- Saving an unsaved snapshot as a named collection now clones runtime materialization from the source snapshot.
- Collection root path keys now respect `mods_path` during persistence so stored keys match runtime normalization.

## Impacted Files

- `src-tauri/src/services/collections/storage.rs` (modified)
- `src-tauri/src/services/collections/apply.rs` (modified)
- `src-tauri/src/services/collections/runtime_snapshot.rs` (modified)
- `src-tauri/src/database/collection_repo.rs` (modified)
- `src-tauri/tests/collection_runtime_materialization.rs` (added)

## Goal

- `collection_roots` and `collection_signatures` are filled immediately when saving a collection or creating a snapshot.

## Impact

- Save/apply flows no longer depend solely on read-time backfill for runtime materialization.
- Strict active collection matching is more stable after save and snapshot creation.
- No schema change in this pass.

## Notes

- Added focused regression coverage for save-current, snapshot creation, and save-snapshot-as-named flows.
