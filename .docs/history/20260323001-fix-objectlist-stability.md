# Fixed ObjectList Disabled State Stability

## Context

ObjectList items were incorrectly shown as disabled (enabled_count=0) after applying a collection because the mod identities (keys and IDs) were unstable—they changed when the mod was toggled (prefixing with `DISABLED `).

## Changes

- **Stabilized Path Keys**: Updated `path_key.rs` to strip the `DISABLED ` prefix during key generation. IDs and matching keys are now the same whether a mod is enabled or disabled.
- **Stable Mod IDs**: Updated `helpers.rs` to generate IDs based on the stable path key.
- **Improved Collection Snapshotting**: Updated `collection_service.rs`, `corridor_service.rs`, and `switch_pipeline.rs` to use the stable `folder_path_key` column for snapshots and previews.
- **Identity Reconciliation**: Expanded the startup migration in `helpers.rs` to automatically fix existing database records (mods, objects, and collection members).

## Impacted Files

- `src-tauri/src/services/path_key.rs` (modified)
- `src-tauri/src/services/scanner/sync/helpers.rs` (modified)
- `src-tauri/src/services/collection_service.rs` (modified)
- `src-tauri/src/services/corridor_service.rs` (modified)
- `src-tauri/src/pipeline/switch_pipeline.rs` (modified)

## Goal

Ensures ObjectList counts and mod identities remain consistent across toggle states and collection applies.

## Impact

- **Fixes**: "Disabled" ObjectList bug after collection apply.
- **Persistence**: Mod metadata (tags/favorites) now persists correctly through toggles.
- **Performance**: Prevents identity-related scanning overhead.
