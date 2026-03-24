# Fix Blank Screen on Corridor Switch (Collections Page)

## Context
When switching corridors (safe ↔ unsafe) from the topbar guard icon or the collections tab buttons, the page was going blank because `selectedId` in `CollectionsPage` is React local state that persisted across corridor switches. After switching, `effectiveSelectedId` still held the old collection ID from the prior corridor. The `useCollectionPreview` query fired with that cross-corridor ID, the backend returned `CollectionError::NotFound`, and React rendered a blank error state.

## Changes
- Added `useEffect` in `CollectionsPage.tsx` that resets `selectedId` and `applyTargetId` to `null` whenever `safeMode` changes.
- The `effectiveSelectedId` will then correctly fall back to `corridor.data?.active_collection_id` (the new corridor's active collection) on the next render cycle.

## Impacted Files
- `src/features/collections/CollectionsPage.tsx` (modified)

## Goal
Corridor switches no longer leave stale cross-corridor collection IDs in local state, preventing blank collection preview panels after safe/unsafe mode toggling.

## Impact
- `useCollectionPreview` will briefly show a loading state during the corridor transition, then resolve to the new corridor's active collection preview.
- No backend changes required.
