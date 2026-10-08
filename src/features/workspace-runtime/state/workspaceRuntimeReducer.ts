import { rewriteWorkspacePathValue } from '../utils/pathRewrite';
import { pathStartsWith } from '@/shared/lib/pathKey';
import type { WorkspaceRuntimeEvent } from './workspaceEvents';
import {
  INITIAL_WORKSPACE_DIALOG_STATE,
  INITIAL_WORKSPACE_PREVIEW_TRANSITION,
  type WorkspaceRuntimeState,
  type WorkspaceTransitionTarget,
} from './workspaceState';
import {
  applyTransitionTarget,
  buildCurrentPath,
  closeDialogIfTargetRemoved,
  queuePreviewTransition,
  shouldGuardPreviewTransition,
  shouldResetDirtyPreviewForReconciliation,
} from './workspaceRuntimeTransitions';

function rewritePendingTarget(
  target: WorkspaceTransitionTarget,
  rewrites: Array<{ oldPath: string; newPath: string }>,
  selectedObjectFolderPath: string | null,
): WorkspaceTransitionTarget {
  if (target.kind === 'selectMod') {
    return {
      ...target,
      path: rewriteWorkspacePathValue(target.path, rewrites) ?? null,
      selectionEffect: target.selectionEffect
        ? {
            ...target.selectionEffect,
            gridSelection: target.selectionEffect.gridSelection?.map(
              (path) => rewriteWorkspacePathValue(path, rewrites) ?? path,
            ),
          }
        : undefined,
    };
  }

  if (target.kind === 'focusObject') {
    return {
      ...target,
      folderPath: rewriteWorkspacePathValue(target.folderPath, rewrites) ?? target.folderPath,
    };
  }

  if (target.kind === 'navigateExplorer') {
    const explorerSubPath =
      rewriteWorkspacePathValue(target.explorerSubPath, rewrites) ?? undefined;
    return {
      ...target,
      explorerSubPath,
      currentPath: buildCurrentPath(selectedObjectFolderPath, explorerSubPath),
    };
  }

  return target;
}

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
      selectionEffect: event.selectionEffect,
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
    const selectionUnchanged =
      event.reconciliationStatus === 'unchanged' &&
      state.selectedObjectFolderPath === event.selectedObjectFolderPath &&
      state.explorerSubPath === event.explorerSubPath &&
      state.selectedModPath === event.selectedModPath &&
      state.currentPath.length === event.currentPath.length &&
      state.currentPath.every((path, index) => path === event.currentPath[index]);
    const resetDirtyPreview = shouldResetDirtyPreviewForReconciliation(state, event);
    return {
      ...state,
      selectedObjectFolderPath: event.selectedObjectFolderPath,
      explorerSubPath: event.explorerSubPath,
      currentPath: event.currentPath,
      selectedModPath: event.selectedModPath,
      previewDirty: resetDirtyPreview ? false : state.previewDirty,
      previewTransition: selectionUnchanged
        ? state.previewTransition
        : INITIAL_WORKSPACE_PREVIEW_TRANSITION,
      dialogState:
        resetDirtyPreview ||
        (!selectionUnchanged && state.dialogState.kind === 'previewUnsavedChanges')
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
    const gridSelection = new Set(
      [...state.gridSelection].map(
        (path) => rewriteWorkspacePathValue(path, event.rewrites) ?? path,
      ),
    );
    const pendingTarget =
      state.previewTransition.kind === 'pending' ? state.previewTransition.pendingTarget : null;
    const pendingSelectionEffect =
      pendingTarget?.kind === 'selectMod' ? pendingTarget.selectionEffect : undefined;
    const callbackPathChanged =
      pendingSelectionEffect?.onApplied !== undefined &&
      (pendingSelectionEffect.affectedPaths ?? []).some(
        (path) => rewriteWorkspacePathValue(path, event.rewrites) !== path,
      );
    const previewTransition =
      state.previewTransition.kind !== 'pending' || callbackPathChanged
        ? callbackPathChanged
          ? INITIAL_WORKSPACE_PREVIEW_TRANSITION
          : state.previewTransition
        : {
            kind: 'pending' as const,
            pendingTarget: rewritePendingTarget(
              state.previewTransition.pendingTarget,
              event.rewrites,
              selectedObjectFolderPath,
            ),
          };

    return {
      ...state,
      selectedObjectFolderPath,
      explorerSubPath,
      selectedModPath,
      gridSelection,
      currentPath: buildCurrentPath(selectedObjectFolderPath, explorerSubPath),
      previewTransition,
      dialogState:
        callbackPathChanged && state.dialogState.kind === 'previewUnsavedChanges'
          ? INITIAL_WORKSPACE_DIALOG_STATE
          : state.dialogState,
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
