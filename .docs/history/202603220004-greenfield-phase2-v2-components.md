# Greenfield Phase 2 (Steps 2.5–2.6): V2 Component Refactoring

## Context

Component layer of the greenfield redesign. Creates v2 page and extracted components using v2 hooks/types — no legacy dependencies.

## Changes

- **V2CollectionsPage** (`V2CollectionsPage.tsx`): 189 lines replacing 658-line original. Eliminates 6 `useMemo` chains, 3 `useEffect` syncs, and `resolveActiveCollection`/`buildCollectionWorkspaceRows` utility calls
- **V2CollectionList** (`components/v2/V2CollectionList.tsx`): Self-contained list with inline edit, rename, apply, delete. Uses `V2CollectionSummary` directly — no `CollectionWorkspaceRow` intermediary
- **V2CollectionPreviewPanel** (`components/v2/V2CollectionPreviewPanel.tsx`): Self-contained preview with `useV2CollectionPreview` hook. Groups members by `object_id` for display. Replaces `CollectionWorkspace` for preview use case

## Impacted Files

- `src/features/collections/V2CollectionsPage.tsx` (added)
- `src/features/collections/components/v2/V2CollectionList.tsx` (added)
- `src/features/collections/components/v2/V2CollectionPreviewPanel.tsx` (added)

## Goal

Complete v2 frontend component layer — page + list + preview ready for integration testing.

## Impact

- No breaking changes — new components coexist with legacy
- `npx tsc --noEmit` passes with zero errors
- Legacy CollectionsPage remains untouched for rollback safety
