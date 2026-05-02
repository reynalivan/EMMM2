# Fix unsaved active state after save then delete

## Context

Deleting the active named collection after saving from an unsaved state could leave the corridor briefly pointing to no active collection. The UI then showed the recreated unsaved row with an `Apply` action instead of `Save`.

## Changes

- Updated collection delete flow to eagerly restore a stable active state when the deleted collection was active.
- If another unsaved collection already exists in the same corridor, it is promoted immediately.
- If no unsaved fallback exists, the service now recreates an unsaved snapshot from live corridor state during delete, instead of waiting for later query side effects.
- Added backend coverage for `unsaved -> save -> delete`, existing-unsaved promotion, and corridor isolation.
- Added a frontend render test proving an active unsaved row shows `Save` and not `Apply`.

## Impacted Files

- `src-tauri/src/services/collection_service.rs` (modified)
- `src/features/collections/components/CollectionList.test.tsx` (added)

## Goal

After deleting the active named collection, the system immediately returns to an active `Unsaved Preset` state when appropriate, so collection actions stay correct.

## Impact

- Collections list no longer depends on query timing or lazy corridor healing for this flow.
- `Save` button behavior for active unsaved rows stays consistent after `save -> delete`.
- No API schema or database migration changes were added.

## Notes

- Frontend tests passed.
- Rust tests compiled successfully, but executing Rust test binaries still fails in this environment with `STATUS_ENTRYPOINT_NOT_FOUND`.
