# ObjectList Architecture Improvements

## Context

Identified and fixed 5 architectural issues in the ObjectList feature: a double-render bug, redundant array creation in search, a stale closure risk, a 40+ flat-property hook return object, and residual eslint-disable comments.

## Changes

### Logic/Behavior (before → after)

1. **Double-render fix**: `prevCategoryFilters` + `setState` in render body → `useEffect` with functional updater. Only triggers re-render if active filters actually change.
2. **Search memo fix**: `allObjects.map()` inside `useEffect` → lifted to `useMemo` as `searchItems`. Eliminates redundant array on every render cycle.
3. **Stable callbacks**: `handleFilterChange` and `handleClearFilters` wrapped in `useCallback` with empty deps.
4. **Namespace return**: Hook now returns `{ state, filters, nav, virtualizer, modals, handlers, bulkSelect }` instead of 40+ flat properties.
5. **Stale closure**: Removed `eslint-disable-next-line react-hooks/exhaustive-deps` in background sync — `handleBackgroundSync` added as proper dep.

## Impacted Files

- `src/features/object-list/useObjectListLogic.ts` (modified — primary)
- `src/features/object-list/ObjectList.tsx` (modified — consumer)
- `src/features/object-list/useObjectListLogic.test.ts` (modified — assertions)
- `src/features/object-list/ObjectListModals.test.tsx` (modified — missing props added)

## Goal

Clean, performant ObjectList hook orchestration with no double-renders, accurate dependency tracking, and a readable namespaced API.

## Impact

- Eliminates double render on category filter change
- Search worker no longer re-sends on unrelated `allObjects` updates
- `tsc --noEmit` passes with exit code 0
- No breaking changes to end-user behavior
- All test interfaces updated to match new structure
