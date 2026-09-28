# Collection active preview respects disabled ancestors

## Context

Current changes could show mods below a `DISABLED` parent as active because a terminal mod's enabled status was used without checking its full path.

## Changes

- Exclude terminal-enabled mods below disabled ancestors from live collection membership, projection, and saved-current-state matching.
- Show only enabled objects with active children in live runtime and apply-current preview trees; saved collection previews retain their stored-state view.
- Add a regression test covering disabled terminal folders, disabled intermediate folders, and disabled parent objects.

## Impacted Files

- `src-tauri/src/modules/collections/adapters/sqlite/live.rs` (modified)
- `src-tauri/src/modules/collections/application/collection/live_state.rs` (modified)
- `src-tauri/src/modules/collections/application/collection/preview.rs` (modified)
- `src-tauri/src/modules/collections/application/runtime/mod.rs` (modified)
- `src-tauri/src/modules/collections/application/runtime/tests.rs` (modified)
- `src-tauri/src/modules/workspace/application/projected_state/members.rs` (modified)
- `docs/history/202609280006-collection-active-preview-ancestor-filter.md` (added)

## Goal

Current changes counts, preview, and Save Current State agree on effectively enabled mods.

## Impact

Disabled ancestor descendants no longer count as active collection members. No filesystem mutation, database migration, or API shape change.
