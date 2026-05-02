# Fix collection preview tree anchor

## Context

Collection preview counted active mods but often rendered empty branches because object anchors and mod paths used different path bases.

## Changes

- `buildModTree` now anchors each object tree from `CollectionObject.path_key` first, then reconstructs relative paths from absolute or relative mod paths.
- Nested folder hierarchy is rebuilt from normalized path segments, with safe basename fallback when no anchor can be reconciled.
- Corridor switch preview now returns object `path_key` as `folder_path`, not object id, so shared tree consumers receive a usable folder anchor.
- Added frontend tests for flat mods, nested variant containers, safe fallback behavior, and tree rendering.
- Added a backend test to lock the `path_key = folder_path` preview contract.

## Impacted Files

- `src/features/collections/utils/buildModTree.ts` (modified)
- `src/features/collections/utils/buildModTree.test.ts` (added)
- `src/features/collections/components/CollectionTreeView.test.tsx` (added)
- `src-tauri/src/services/corridor_service.rs` (modified)

## Goal

Collection preview now shows active flat mods and active mods inside variant containers as a readable tree with correct folder hierarchy.

## Impact

- Improves collection preview and other shared tree consumers that depend on object path anchors.
- No schema or migration changes.
- Backend test binary still crashes at runtime in this environment, but the new test compiles successfully with `cargo test --no-run`.
