import type { WorkspaceRuntimeEvent } from '../../features/workspace-runtime/state/workspaceEvents';
import {
  INITIAL_WORKSPACE_DIALOG_STATE,
  INITIAL_WORKSPACE_PREVIEW_TRANSITION,
  type WorkspaceRuntimeState,
  type WorkspaceTransitionTarget,
} from '../../features/workspace-runtime/state/workspaceState';

function getWorkspaceObjectDisplayName(folderPath: string): string {
  const segments = folderPath.split(/[\\/]/).filter(Boolean);
  if (segments.length === 0) {
    return folderPath;
  }

  return segments[segments.length - 1];
}

export function buildCurrentPath(
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

export function shouldGuardPreviewTransition(
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

export function queuePreviewTransition(
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

export function applyTransitionTarget(
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

export function closeDialogIfTargetRemoved(
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

export function shouldResetDirtyPreviewForReconciliation(
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
