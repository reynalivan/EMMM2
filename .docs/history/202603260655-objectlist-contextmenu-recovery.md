# Recover ObjectList Context Menu Baseline

## Context

Context menu ObjectList drifted from the older working baseline and mixed object-only behavior with dead folder branches, causing incorrect menu structure and stale tests.

## Changes

- Recovered the object-row context menu structure to match the older baseline while keeping current handler logic and modern context-menu primitives.
- Removed dead folder-only menu contract from the ObjectList context menu path and narrowed the payload/type to real object-row usage.
- Added a shared object context-menu target builder so normal rows and sticky selected rows emit identical menu payloads.
- Updated menu tests to cover pin/unpin, enable/disable exclusivity, move-category, reveal/edit/sync/delete wiring, and sync disabled state.
- Added content-level verification that row and sticky-row context menus use the same target payload.
- Added missing `objects.context.sync_db` locale keys.

## Impacted Files

- `src/features/object-list/ObjectContextMenu.tsx` (modified)
- `src/features/object-list/ObjectContextMenu.test.tsx` (modified)
- `src/features/object-list/ObjectContextMenuTarget.ts` (added)
- `src/features/object-list/ObjectList.tsx` (modified)
- `src/features/object-list/ObjectListContent.tsx` (modified)
- `src/features/object-list/ObjectListContent.test.tsx` (modified)
- `src/locales/en/objects.json` (modified)
- `src/locales/id/objects.json` (modified)
- `src/locales/zh/objects.json` (modified)

## Goal

ObjectList object rows now expose the expected legacy-compatible menu actions with current backend behavior, and row/sticky menu payloads stay consistent.

## Impact

- No public API expansion; this is a focused UI contract recovery.
- Test coverage for the recovered menu behavior is stronger and now checks the real object-only flow.
- Existing `ObjectList.test.tsx` smoke failures remain in unrelated background-sync and stale i18n assertions; not changed in this recovery.
