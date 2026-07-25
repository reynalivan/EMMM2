import type { WorkspaceRuntimeEvent } from '../../features/workspace-runtime/state/workspaceEvents';
import {
  INITIAL_WORKSPACE_DIALOG_STATE,
  INITIAL_WORKSPACE_PREVIEW_TRANSITION,
  type WorkspaceDialogState,
  type WorkspacePreviewTransitionState,
  type WorkspaceRuntimeState,
} from '../../features/workspace-runtime/state/workspaceState';
import type { AppState } from '../useAppStore';
import type { AppSliceCreator } from './sliceTypes';
import { reduceWorkspaceRuntimeState } from './workspaceRuntimeReducer';

export interface WorkspaceRuntimeSlice {
  // Workspace preview/dialog state, driven by dispatched runtime events.
  workspacePreviewDirty: boolean;
  workspacePreviewTransition: WorkspacePreviewTransitionState;
  workspaceDialogState: WorkspaceDialogState;

  dispatchWorkspaceRuntime: (event: WorkspaceRuntimeEvent) => WorkspaceRuntimeState;
}

export function selectWorkspaceRuntimeState(state: AppState): WorkspaceRuntimeState {
  return {
    selectedObjectFolderPath: state.selectedObjectFolderPath,
    explorerSubPath: state.explorerSubPath,
    currentPath: state.currentPath,
    selectedModPath: state.selectedModPath,
    mobileActivePane: state.mobileActivePane,
    previewDirty: state.workspacePreviewDirty,
    previewTransition: state.workspacePreviewTransition ?? INITIAL_WORKSPACE_PREVIEW_TRANSITION,
    dialogState: state.workspaceDialogState ?? INITIAL_WORKSPACE_DIALOG_STATE,
  };
}

function toAppStatePatch(runtimeState: WorkspaceRuntimeState): Partial<AppState> {
  return {
    selectedObjectFolderPath: runtimeState.selectedObjectFolderPath,
    explorerSubPath: runtimeState.explorerSubPath,
    currentPath: runtimeState.currentPath,
    selectedModPath: runtimeState.selectedModPath,
    mobileActivePane: runtimeState.mobileActivePane,
    workspacePreviewDirty: runtimeState.previewDirty,
    workspacePreviewTransition: runtimeState.previewTransition,
    workspaceDialogState: runtimeState.dialogState,
  };
}

export const createWorkspaceRuntimeSlice: AppSliceCreator<WorkspaceRuntimeSlice> = (set, get) => ({
  workspacePreviewDirty: false,
  workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
  workspaceDialogState: { kind: 'none' },

  dispatchWorkspaceRuntime: (event) => {
    const nextState = reduceWorkspaceRuntimeState(selectWorkspaceRuntimeState(get()), event);
    set(toAppStatePatch(nextState));
    return nextState;
  },
});
