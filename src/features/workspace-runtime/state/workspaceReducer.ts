import type { WorkspaceRuntimeEvent } from './workspaceEvents';
import {
  INITIAL_WORKSPACE_DIALOG_STATE,
  INITIAL_WORKSPACE_PREVIEW_TRANSITION,
  type WorkspaceRuntimeState,
  type WorkspaceTransitionTarget,
} from './workspaceState';

function getWorkspaceObjectDisplayName(folderPath: string): string {
  const segments = folderPath.split(/[\\/]/).filter(Boolean);
  if (segments.length === 0) {
    return folderPath;
  }

  return segments[segments.length - 1];
}

function buildCurrentPath(
  selectedObjectFolderPath: string | null,
  explorerSubPath: string | undefined,
): string[] {
  if (!explorerSubPath) {
    return [];
  }

  if (!selectedObjectFolderPath) {
    return explorerSubPath.split(/[\\/]/).filter(Boolean);
  }

  const rootName = getWorkspaceObjectDisplayName(selectedObjectFolderPath);
  if (explorerSubPath === selectedObjectFolderPath) {
    return [rootName];
  }

  const prefix = `${selectedObjectFolderPath.replace(/\\/g, '/')}/`;
  const normalizedSubPath = explorerSubPath.replace(/\\/g, '/');
  const relative = normalizedSubPath.startsWith(prefix)
    ? normalizedSubPath.slice(prefix.length)
    : normalizedSubPath;

  const suffixSegments = relative.split('/').filter(Boolean);
  return [rootName, ...suffixSegments];
}

function rewritePathValue(
  value: string | null | undefined,
  rewrites: Array<{ oldPath: string; newPath: string }>,
): string | null | undefined {
  if (!value) {
    return value;
  }

  let nextValue = value.replace(/\\/g, '/');
  for (const rewrite of rewrites) {
    const oldPath = rewrite.oldPath.replace(/\\/g, '/');
    const newPath = rewrite.newPath.replace(/\\/g, '/');
    const oldName = oldPath.split('/').filter(Boolean).pop() ?? oldPath;
    const newName = newPath.split('/').filter(Boolean).pop() ?? newPath;
    if (nextValue === oldPath) {
      nextValue = newPath;
      continue;
    }

    if (nextValue.startsWith(`${oldPath}/`)) {
      nextValue = `${newPath}${nextValue.slice(oldPath.length)}`;
      continue;
    }

    if (nextValue === oldName) {
      nextValue = newName;
      continue;
    }

    if (nextValue.startsWith(`${oldName}/`)) {
      nextValue = `${newName}${nextValue.slice(oldName.length)}`;
      continue;
    }

    const segments = nextValue.split('/');
    if (segments.includes(oldName)) {
      nextValue = segments.map((segment) => (segment === oldName ? newName : segment)).join('/');
    }
  }

  return nextValue;
}

function shouldGuardPreviewTransition(
  state: WorkspaceRuntimeState,
  target: WorkspaceTransitionTarget,
): boolean {
  if (!state.previewDirty) {
    return false;
  }

  if (state.previewTransition.kind === 'pending') {
    return false;
  }

  if (!state.selectedModPath) {
    return false;
  }

  if (target.kind === 'selectMod') {
    return target.path !== state.selectedModPath;
  }

  if (target.kind === 'clearSelection') {
    return true;
  }

  if (target.kind === 'focusObject') {
    return true;
  }

  if (target.kind === 'navigateExplorer') {
    return target.explorerSubPath !== state.explorerSubPath;
  }

  if (target.kind === 'collapseSection') {
    return false;
  }

  return false;
}

function queuePreviewTransition(
  state: WorkspaceRuntimeState,
  target: WorkspaceTransitionTarget,
): WorkspaceRuntimeState {
  return {
    ...state,
    previewTransition: {
      kind: 'pending',
      pendingTarget: target,
    },
    dialogState: { kind: 'previewUnsavedChanges' },
  };
}

function applyTransitionTarget(
  state: WorkspaceRuntimeState,
  target: WorkspaceTransitionTarget,
): WorkspaceRuntimeState {
  if (target.kind === 'focusObject') {
    return {
      ...state,
      selectedObjectFolderPath: target.folderPath,
      explorerSubPath: target.folderPath,
      currentPath: [getWorkspaceObjectDisplayName(target.folderPath)],
      selectedModPath: null,
      mobileActivePane: 'grid',
      previewTransition: INITIAL_WORKSPACE_PREVIEW_TRANSITION,
      dialogState:
        state.dialogState.kind === 'previewUnsavedChanges'
          ? INITIAL_WORKSPACE_DIALOG_STATE
          : state.dialogState,
    };
  }

  if (target.kind === 'navigateExplorer') {
    return {
      ...state,
      currentPath: target.currentPath,
      explorerSubPath: target.explorerSubPath,
      selectedModPath: null,
      previewTransition: INITIAL_WORKSPACE_PREVIEW_TRANSITION,
      dialogState:
        state.dialogState.kind === 'previewUnsavedChanges'
          ? INITIAL_WORKSPACE_DIALOG_STATE
          : state.dialogState,
    };
  }

  if (target.kind === 'selectMod') {
    return {
      ...state,
      selectedModPath: target.path,
      mobileActivePane: target.mobilePane ?? state.mobileActivePane,
      previewTransition: INITIAL_WORKSPACE_PREVIEW_TRANSITION,
      dialogState:
        state.dialogState.kind === 'previewUnsavedChanges'
          ? INITIAL_WORKSPACE_DIALOG_STATE
          : state.dialogState,
    };
  }

  if (target.kind === 'collapseSection') {
    return {
      ...state,
      previewTransition: INITIAL_WORKSPACE_PREVIEW_TRANSITION,
      dialogState:
        state.dialogState.kind === 'previewUnsavedChanges'
          ? INITIAL_WORKSPACE_DIALOG_STATE
          : state.dialogState,
    };
  }

  return {
    ...state,
    selectedObjectFolderPath:
      target.clearObjectSelection === false ? state.selectedObjectFolderPath : null,
    selectedModPath: null,
    explorerSubPath: target.resetExplorer ? undefined : state.explorerSubPath,
    currentPath: target.resetExplorer ? [] : state.currentPath,
    mobileActivePane: target.mobilePane ?? state.mobileActivePane,
    previewTransition: INITIAL_WORKSPACE_PREVIEW_TRANSITION,
    dialogState:
      state.dialogState.kind === 'previewUnsavedChanges'
        ? INITIAL_WORKSPACE_DIALOG_STATE
        : state.dialogState,
  };
}

function closeDialogIfTargetRemoved(
  state: WorkspaceRuntimeState,
  invalidPaths: string[],
): WorkspaceRuntimeState['dialogState'] {
  if (state.dialogState.kind === 'none' || state.dialogState.kind === 'previewUnsavedChanges') {
    return state.dialogState;
  }

  const targetPath =
    state.dialogState.kind === 'conflict'
      ? state.dialogState.conflict.attempted_target
      : state.dialogState.kind === 'fileInUse'
        ? state.dialogState.data.path
        : state.dialogState.kind === 'folderEnableParent'
          ? state.dialogState.ancestorPath
          : 'folder' in state.dialogState
            ? state.dialogState.folder.path
            : null;
  if (!targetPath) {
    return state.dialogState;
  }

  const normalizedTargetPath = targetPath.replace(/\\/g, '/');
  const hit = invalidPaths.some((path) => {
    const normalizedPath = path.replace(/\\/g, '/');
    return (
      normalizedTargetPath === normalizedPath ||
      normalizedTargetPath.startsWith(`${normalizedPath}/`)
    );
  });

  if (!hit) {
    return state.dialogState;
  }

  return INITIAL_WORKSPACE_DIALOG_STATE;
}

function pathTouchesTarget(targetPath: string, affectedPaths: string[]): boolean {
  const normalizedTargetPath = targetPath.replace(/\\/g, '/');
  return affectedPaths.some((path) => {
    const normalizedPath = path.replace(/\\/g, '/');
    return (
      normalizedTargetPath === normalizedPath ||
      normalizedTargetPath.startsWith(`${normalizedPath}/`) ||
      normalizedPath.startsWith(`${normalizedTargetPath}/`)
    );
  });
}

function shouldResetDirtyPreviewForReconciliation(
  state: WorkspaceRuntimeState,
  event: Extract<WorkspaceRuntimeEvent, { type: 'SELECTION_RECONCILED' }>,
): boolean {
  if (!state.previewDirty || event.reconciliationStatus === 'unchanged') {
    return false;
  }

  if (!state.selectedModPath) {
    return false;
  }

  if (state.selectedModPath !== event.selectedModPath) {
    return true;
  }

  return pathTouchesTarget(state.selectedModPath, event.affectedPaths);
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
      rewritePathValue(state.selectedObjectFolderPath, event.rewrites) ?? null;
    const explorerSubPath = rewritePathValue(state.explorerSubPath, event.rewrites) ?? undefined;
    const selectedModPath = rewritePathValue(state.selectedModPath, event.rewrites) ?? null;

    return {
      ...state,
      selectedObjectFolderPath,
      explorerSubPath,
      selectedModPath,
      currentPath: buildCurrentPath(selectedObjectFolderPath, explorerSubPath),
    };
  }

  if (event.type === 'TARGETS_INVALIDATED') {
    const invalidPaths = event.paths.map((path) => path.replace(/\\/g, '/'));
    const normalizedObjectPath = state.selectedObjectFolderPath?.replace(/\\/g, '/');
    const normalizedModPath = state.selectedModPath?.replace(/\\/g, '/');
    const objectInvalid =
      !!normalizedObjectPath &&
      invalidPaths.some(
        (path) => normalizedObjectPath === path || normalizedObjectPath.startsWith(`${path}/`),
      );
    const modInvalid =
      !!normalizedModPath &&
      invalidPaths.some(
        (path) => normalizedModPath === path || normalizedModPath.startsWith(`${path}/`),
      );
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
        : closeDialogIfTargetRemoved(state, invalidPaths),
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
