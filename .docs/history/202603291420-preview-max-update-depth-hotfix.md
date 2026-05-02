# Preview Max Update Depth Hotfix

## Context

Preview panel could hit React `Maximum update depth exceeded` after the runtime/preview refactor because effect dependencies were watching unstable derived arrays and then writing equivalent state back on every render.

## Changes

- Stabilized runtime preview selector usage to read primitive slices separately instead of creating a new selector object each render.
- Added equality guards for preview editor field maps and open section sets before calling `setState`.
- Removed unstable array/object dependencies from preview state reset effects and keyed them off stable identifiers only.
- Prevented redundant `PREVIEW_DIRTY_CHANGED` dispatches when runtime dirty state already matches derived dirty state.

## Impacted Files

- `src/features/preview/hooks/usePreviewPanelState.ts` (modified)

## Goal

Preview state synchronization no longer re-enters itself during normal render/update cycles, so the panel can mount and react to query changes without infinite nested updates.

## Impact

- Fixes the immediate React infinite-update crash in preview flows.
- Keeps existing preview dirty, section toggle, and editor reset behavior intact.
- No API or data contract changes.

## Notes

- The bug came from state writes guarded only by effect execution, not by value changes. The hotfix keeps the refactor structure and tightens those guards.
