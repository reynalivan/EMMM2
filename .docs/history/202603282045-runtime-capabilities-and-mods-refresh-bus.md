# Runtime Capabilities and Mods Refresh Bus

## Context

`mods` runtime still had two architecture gaps: action availability was inferred in UI, and several refresh paths still bypassed the runtime event bus with ad-hoc object refresh calls.

## Changes

- Backend `WorkspaceViewModel` now emits action capability metadata for object rows and explorer nodes.
- Frontend workspace types now carry shared runtime capabilities on object rows and explorer nodes.
- Object and mod context menus now read capability metadata instead of relying only on local handler presence.
- `FolderListRow` moved onto the shared mod context-menu policy so list/grid menus stop drifting.
- Runtime refresh helpers now route object/folder refresh through event scopes instead of direct query invalidation paths.
- Watcher reconcile now publishes runtime events for workspace/object/folder/thumbnail/dashboard/conflict refresh.
- Preview detail mutations now publish `previewChanged`, `folderMetadataChanged`, or `thumbnailChanged` events after targeted cache invalidation.
- Thumbnail import from mod context menu no longer depends on direct filesystem reads; it now uses the existing thumbnail update command path.

## Impacted Files

- Backend runtime contract:
  - `src-tauri/src/domain/workspace.rs` (modified)
  - `src-tauri/src/services/workspace_service.rs` (modified)
- Frontend runtime/types:
  - `src/types/workspace.ts` (modified)
  - `src/features/runtime-sync/queryRefresh.ts` (modified)
  - `src/hooks/useObjects.ts` (modified)
- UI consumers:
  - `src/features/object-list/ObjectContextMenu.tsx` (modified)
  - `src/features/object-list/ObjectContextMenuTarget.ts` (modified)
  - `src/hooks/useModContextMenuItems.ts` (modified)
  - `src/features/preview/components/PreviewPanelContextMenu.tsx` (modified)
  - `src/features/folder-grid/FolderListRow.tsx` (modified)
  - `src/features/folder-grid/FolderGrid.tsx` (modified)
- Runtime refresh/mutation flows:
  - `src/features/preview/hooks/usePreviewData.ts` (modified)
  - `src/features/file-watcher/hooks.ts` (modified)
  - `src/features/object-list/useObjHandlersBulk.ts` (modified)
  - `src/features/workspace-runtime/actions/sharedObjectActionOps.ts` (modified)
  - `src/features/workspace-runtime/actions/useSharedObjectActions.ts` (modified)
- Tests:
  - `src/features/folder-grid/FolderCard.test.tsx` (modified)
  - `src/features/folder-grid/FolderCardContextMenu.test.tsx` (modified)
  - `src/features/folder-grid/FolderListRow.test.tsx` (modified)
  - `src/features/object-list/ObjectContextMenu.test.tsx` (modified)
  - `src/features/object-list/ObjectListContent.test.tsx` (modified)
  - `src/features/object-list/ObjectRowItem.test.tsx` (modified)
  - `src/features/object-list/useObjectListHandlers.test.ts` (modified)
  - `src/features/file-watcher/hooks.test.ts` (modified)

## Goal

`mods` runtime now carries backend-authored action capabilities and uses the event-scoped refresh bus more consistently, so ObjectList, FolderGrid, Preview, and watcher-driven refresh are less dependent on local inference and ad-hoc invalidation.

## Impact

- Context-menu visibility is now driven by runtime contract data, reducing surface drift between grid/list/preview/object menus.
- Watcher and preview-detail mutations refresh the same runtime scopes as other workspace flows.
- Thumbnail import behavior is simpler and more test-friendly.
- No DB migration or user-facing workflow change.

## Notes

- This closes more of Phase A and Phase B, but global workspace state machine, optimistic descriptors, and final legacy cleanup are still follow-up work.
