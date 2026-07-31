import { rewriteWorkspacePathValue } from '../../features/workspace-runtime/pathRewrite';
import { pathStartsWith } from '../../lib/pathKey';
import type { WorkspaceRuntimeEvent } from '../../features/workspace-runtime/state/workspaceEvents';
import {
  INITIAL_WORKSPACE_DIALOG_STATE,
  INITIAL_WORKSPACE_PREVIEW_TRANSITION,
  type WorkspaceRuntimeState,
  type WorkspaceTransitionTarget,
} from '../../features/workspace-runtime/state/workspaceState';
import {
  applyTransitionTarget,
  buildCurrentPath,
  closeDialogIfTargetRemoved,
  queuePreviewTransition,
  shouldGuardPreviewTransition,
  shouldResetDirtyPreviewForReconciliation,
} from './workspaceRuntimeTransitions';

export function reduceWorkspaceRuntimeState(
  state: WorkspaceRuntimeState,
  event: WorkspaceRuntimeEvent,
): WorkspaceRuntimeState {
  if (event.type === 'OBJECT_FOCUSED') {
    const target: WorkspaceTransitionTarget = { kind: 'focusObject', folderPath: event.folderPath };
    if (shouldGuardPreviewTransition(state, target)) {
      return queuePreviewTransition(state, target);
    }
    return applyTransitionTarget(state, target);
  }

  if (event.type === 'OBJECT_CLEARED' || event.type === 'SELECTION_CLEARED') {
    const target: WorkspaceTransitionTarget = {
      kind: 'clearSelection',
      resetExplorer: event.resetExplorer,
      mobilePane: event.mobilePane,
      clearObjectSelection: 'clearObjectSelection' in event ? event.clearObjectSelection : true,
    };
    if ('force' in event && event.force) {
      return applyTransitionTarget(state, target);
    }
    if (shouldGuardPreviewTransition(state, target)) {
      return queuePreviewTransition(state, target);
    }
    return applyTransitionTarget(state, target);
  }

  if (event.type === 'EXPLORER_NAVIGATED') {
    const target: WorkspaceTransitionTarget = {
      kind: 'navigateExplorer',
      currentPath: event.currentPath,
      explorerSubPath: event.explorerSubPath,
    };
    if (shouldGuardPreviewTransition(state, target)) {
      return queuePreviewTransition(state, target);
    }
    return applyTransitionTarget(state, target);
  }

  if (event.type === 'MOD_SELECTED') {
    const target: WorkspaceTransitionTarget = {
      kind: 'selectMod',
      path: event.path,
      mobilePane: event.mobilePane,
    };
    if (shouldGuardPreviewTransition(state, target)) {
      return queuePreviewTransition(state, target);
    }
    return applyTransitionTarget(state, target);
  }

  if (event.type === 'PREVIEW_DIRTY_CHANGED') {
    return {
      ...state,
      previewDirty: event.dirty,
    };
  }

  if (event.type === 'PREVIEW_TRANSITION_REQUESTED') {
    return queuePreviewTransition(state, event.target);
  }

  if (event.type === 'PREVIEW_TRANSITION_CONFIRMED') {
    if (state.previewTransition.kind !== 'pending') {
      return state;
    }

    return applyTransitionTarget(
      {
        ...state,
        previewDirty: false,
      },
      state.previewTransition.pendingTarget,
    );
  }

  if (event.type === 'PREVIEW_TRANSITION_CANCELLED') {
    return {
      ...state,
      previewTransition: INITIAL_WORKSPACE_PREVIEW_TRANSITION,
      dialogState:
        state.dialogState.kind === 'previewUnsavedChanges'
          ? INITIAL_WORKSPACE_DIALOG_STATE
          : state.dialogState,
    };
  }

  if (event.type === 'SELECTION_RECONCILED') {
    const resetDirtyPreview = shouldResetDirtyPreviewForReconciliation(state, event);
    return {
      ...state,
      selectedObjectFolderPath: event.selectedObjectFolderPath,
      explorerSubPath: event.explorerSubPath,
      currentPath: event.currentPath,
      selectedModPath: event.selectedModPath,
      previewDirty: resetDirtyPreview ? false : state.previewDirty,
      previewTransition: INITIAL_WORKSPACE_PREVIEW_TRANSITION,
      dialogState:
        resetDirtyPreview || state.dialogState.kind === 'previewUnsavedChanges'
          ? INITIAL_WORKSPACE_DIALOG_STATE
          : state.dialogState,
    };
  }

  if (event.type === 'PATHS_REWRITTEN') {
    const selectedObjectFolderPath =
      rewriteWorkspacePathValue(state.selectedObjectFolderPath, event.rewrites) ?? null;
    const explorerSubPath =
      rewriteWorkspacePathValue(state.explorerSubPath, event.rewrites) ?? undefined;
    const selectedModPath =
      rewriteWorkspacePathValue(state.selectedModPath, event.rewrites) ?? null;

    return {
      ...state,
      selectedObjectFolderPath,
      explorerSubPath,
      selectedModPath,
      currentPath: buildCurrentPath(selectedObjectFolderPath, explorerSubPath),
    };
  }

  if (event.type === 'TARGETS_INVALIDATED') {
    const objectInvalid = event.paths.some((path) =>
      pathStartsWith(path, state.selectedObjectFolderPath),
    );
    const modInvalid = event.paths.some((path) => pathStartsWith(path, state.selectedModPath));
    const previewTargetInvalid = objectInvalid || modInvalid;

    return {
      ...state,
      selectedObjectFolderPath: objectInvalid ? null : state.selectedObjectFolderPath,
      selectedModPath: previewTargetInvalid ? null : state.selectedModPath,
      explorerSubPath: objectInvalid && event.resetExplorer ? undefined : state.explorerSubPath,
      currentPath: objectInvalid && event.resetExplorer ? [] : state.currentPath,
      previewDirty: previewTargetInvalid ? false : state.previewDirty,
      previewTransition: INITIAL_WORKSPACE_PREVIEW_TRANSITION,
      dialogState: previewTargetInvalid
        ? INITIAL_WORKSPACE_DIALOG_STATE
        : closeDialogIfTargetRemoved(state, event.paths),
    };
  }

  if (event.type === 'DIALOG_OPENED' || event.type === 'DIALOG_UPDATED') {
    return {
      ...state,
      dialogState: event.dialog,
    };
  }

  if (event.type === 'DIALOG_CLOSED') {
    if (!event.kind || state.dialogState.kind === event.kind) {
      return {
        ...state,
        dialogState: INITIAL_WORKSPACE_DIALOG_STATE,
      };
    }

    return state;
  }

  return state;
}
