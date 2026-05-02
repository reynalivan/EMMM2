# Shared Object Actions and Runtime Selection

## Context

Object-side mutations and workspace selection transitions were still drifting across `ObjectList`, `FolderGrid`, and `PreviewPanel`. This kept object actions outside the shared runtime action layer and left navigation/selection state updates inconsistent.

## Changes

- Moved object CRUD/sync/toggle/pin/category/reveal behavior into a shared runtime hook:
  - object dialogs and sync confirm now live in one reducer-backed action engine
  - object DB sync no longer lives inside scan handlers
- Added shared runtime selection transition helpers:
  - focus object root
  - apply explorer location
  - clear workspace selection consistently
- Rewired `ObjectList`, `FolderGrid`, `PreviewPanel`, and rename path sync to use the shared selection transitions instead of ad-hoc store writes.
- Simplified object-list orchestration:
  - scan hook now only owns scan/reconcile flow
  - CRUD hook becomes a thin adapter over the shared object action engine

## Impacted Files

- `src/features/workspace-runtime/runtimeSelection.ts` (added)
- `src/features/workspace-runtime/actions/useSharedObjectActions.ts` (added)
- `src/features/workspace-runtime/actions/useSharedObjectSyncActions.ts` (added)
- `src/features/workspace-runtime/actions/sharedObjectActionsState.ts` (added)
- `src/features/workspace-runtime/actions/sharedObjectActionOps.ts` (added)
- `src/features/object-list/useObjHandlersCrud.ts` (modified)
- `src/features/object-list/useObjHandlersScan.ts` (modified)
- `src/features/object-list/useObjectListHandlers.ts` (modified)
- `src/features/object-list/useObjectListLogic.ts` (modified)
- `src/features/object-list/ObjectList.tsx` (modified)
- `src/features/object-list/ObjectListContent.tsx` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridNav.ts` (modified)
- `src/features/preview/PreviewPanel.tsx` (modified)
- `src/features/mod-runtime/operations/sharedOperations.ts` (modified)
- `src/features/object-list/ObjectListContent.test.tsx` (modified)
- `src/features/object-list/useObjectListLogic.test.ts` (modified)
- `src/features/object-list/useObjectListHandlers.test.ts` (modified)

## Goal

Object actions now follow the same runtime-action direction as mod actions, and workspace selection/navigation updates are applied through one deterministic transition layer.

## Impact

- `ObjectList` object actions are less coupled to scan logic and easier to extend into the full shared action engine roadmap.
- `FolderGrid` and `PreviewPanel` now clear or sync workspace selection state more consistently.
- No DB/schema changes.
- Remaining work is still needed for a full workspace state machine and broader declarative optimistic descriptor system.
