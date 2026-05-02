## Title

Global workspace state machine and optimistic descriptor foundation

## Context

Selection, navigation, preview unsaved transitions, and optimistic runtime patches for the `mods` workspace were still split across hooks and local reducers. That caused drift between `ObjectList`, `FolderGrid`, `PreviewPanel`, and shared action dialogs.

## Changes

- Added a typed workspace runtime state/reducer layer for selection, navigation, preview transition, dialog state, and drag/drop state.
- Moved main `mods` selection/navigation consumers to dispatch runtime events instead of mutating store selection fields directly.
- Moved shared mod/object dialog state to the workspace machine bridge instead of local reducers.
- Added runtime effect descriptors for path rewrites, invalidations, object count deltas, and thumbnail refresh effects.
- Applied descriptor-driven optimistic effects to key folder mutations and routed descriptor refresh publishing through the runtime refresh bus.
- Added reducer and optimistic-effect tests for transition correctness and deterministic cache patching.

## Impacted Files

- `.docs/history/202603292205-global-workspace-state-machine-and-optimistic-descriptor.md` (added)

- `src/features/workspace-runtime/state/workspaceState.ts` (added)
- `src/features/workspace-runtime/state/workspaceEvents.ts` (added)
- `src/features/workspace-runtime/state/workspaceReducer.ts` (added)
- `src/features/workspace-runtime/state/workspaceSelectors.ts` (added)
- `src/features/workspace-runtime/state/workspaceStoreBridge.ts` (added)

- `src/features/workspace-runtime/optimistic/descriptor.ts` (added)
- `src/features/workspace-runtime/optimistic/descriptorBuilders.ts` (added)
- `src/features/workspace-runtime/optimistic/applyOptimisticEffects.ts` (added)

- `src/features/workspace-runtime/state/workspaceReducer.test.ts` (added)
- `src/features/workspace-runtime/optimistic/applyOptimisticEffects.test.ts` (added)

- `src/stores/useAppStore.ts` (modified)
- `src/features/workspace-runtime/useWorkspaceViewModel.ts` (modified)
- `src/features/object-list/useObjectListLogic.ts` (modified)
- `src/features/object-list/ObjectList.tsx` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGridNav.ts` (modified)
- `src/features/preview/PreviewPanel.tsx` (modified)
- `src/features/preview/hooks/usePreviewPanelState.ts` (modified)
- `src/features/mod-runtime/operations/sharedOperations.ts` (modified)
- `src/features/mod-runtime/actions/useSharedModActions.ts` (modified)
- `src/features/workspace-runtime/actions/useSharedObjectActions.ts` (modified)
- `src/features/runtime-sync/queryRefresh.ts` (modified)
- `src/hooks/useFolders.ts` (modified)
- `src/hooks/useObjects.ts` (modified)
- `src/features/preview/hooks/usePreviewPanelState.test.ts` (modified)
- `src/features/folder-grid/hooks/useFolderGrid.test.ts` (modified)
- `src/features/object-list/useObjectListLogic.test.ts` (modified)
- `src/hooks/useObjects.test.tsx` (modified)

## Goal

The `mods` workspace now has a central runtime state model for selection and dialog transitions, plus a descriptor-based path/count optimistic layer that can be shared by runtime actions and refresh publishing.

## Impact

- `ObjectList`, `FolderGrid`, and `PreviewPanel` are less likely to drift during object focus, mod selection, rename/move path rewrites, and preview-unsaved transitions.
- Optimistic path rewrite and object count updates are more auditable because they now flow through explicit descriptors.
- Breaking changes: none intended for user-facing behavior, but internal selection/dialog plumbing changed for `mods` runtime consumers.

## Notes

- This is a foundation phase. Not every mutation path in the repo is descriptor-driven yet, and editor-local section-collapse transitions remain local to the preview hook.
