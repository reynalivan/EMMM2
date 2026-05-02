# Close Remaining ObjectList Responsive Gaps

## Context

ObjectList still had stale refresh paths, duplicated mutation logic, and stale requirement docs even after the earlier responsiveness fixes. Some flows only refreshed after settle, pin order did not move immediately, and watcher/object actions were easy to drift out of sync.

## Changes

- Standardized ObjectList refresh paths around shared object-query helpers and reused them across object CRUD, bulk handlers, FolderGrid, Preview, and watcher flows.
- Added optimistic object-row handling for pin state, object-root enable/disable, and deterministic single-mod enabled-count updates before active refetch settles.
- Consolidated duplicated folder DB sync and move-to-object flows into shared helpers so FolderGrid, Preview, and ObjectList scan paths refresh the same way.
- Removed dead/legacy surface pieces that no longer had a real consumer (`handleDropNewObject`, `masterDb: null`, stale handler-map exports).
- Updated virtualized ordering so pinned items render at the top of their section immediately.
- Polished row visuals so `enabled_count = 0` objects read as explicitly inactive while keeping the count chip muted.
- Synced requirements with the current runtime model: `selectedObjectFolderPath`, thumbnail-backed rows, optimistic object edits, and watcher-driven ObjectList refresh triggers.

## Impacted Files

- `.docs/requirements/req-07-object-list.md` (modified)
- `.docs/requirements/req-10-object-crud.md` (modified)
- `.docs/requirements/req-28-file-watcher.md` (modified)
- `src/hooks/useObjects.ts` (modified)
- `src/hooks/useObjects.test.tsx` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src/features/object-list/objHandlersHelpers.ts` (modified)
- `src/features/object-list/useObjHandlersCrud.ts` (modified)
- `src/features/object-list/useObjHandlersBulk.ts` (modified)
- `src/features/object-list/useObjHandlersDrop.ts` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/object-list/useObjectListHandlers.ts` (modified)
- `src/features/object-list/useObjectListHandlers.test.ts` (modified)
- `src/features/object-list/useObjectListLogic.ts` (modified)
- `src/features/object-list/useObjectListLogic.test.ts` (modified)
- `src/features/object-list/useObjectListVirtualizer.ts` (modified)
- `src/features/object-list/useObjectListVirtualizer.test.ts` (modified)
- `src/features/object-list/ObjectRowItem.tsx` (modified)
- `src/features/object-list/ObjectRowItem.test.tsx` (modified)
- `src/features/object-list/hooks/useMasterDbSync.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridActions.ts` (modified)
- `src/features/preview/hooks/usePreviewPanelActions.ts` (modified)
- `src/features/file-watcher/hooks.test.ts` (modified)

## Goal

ObjectList now responds immediately and consistently to object edits, pin/unpin, object-root enable/disable, deterministic mod toggles, watcher path updates, and shared move/sync flows without depending on stale one-off invalidation logic.

## Impact

- Pin/unpin now visibly reorders rows immediately inside each category.
- FolderGrid, Preview, and watcher flows use the same ObjectList refresh policy, reducing future drift.
- Single-mod toggle optimistic count patch is limited to deterministic non-container nodes; ambiguous container cases still rely on backend-authoritative refresh.
- No Rust API or schema changes were required.

## Notes

- `selectedObject` plumbing inside FolderGrid nav still exists as legacy shape, but the stale placeholder comments were removed and the ObjectList-facing requirements now reflect `selectedObjectFolderPath` as the real source of truth.
