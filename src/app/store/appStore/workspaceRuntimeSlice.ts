import type { WorkspaceRuntimeEvent } from '@/features/workspace-runtime';
import type {
  WorkspaceDialogState,
  WorkspacePreviewTransitionState,
  WorkspaceRuntimeState,
} from '@/features/workspace-runtime';
import type { AppState } from '../useAppStore';
import type { AppSliceCreator } from './sliceTypes';
import {
  reduceWorkspaceRuntimeState,
  selectWorkspaceRuntimeState,
} from '@/features/workspace-runtime/@x/app';

export interface WorkspaceRuntimeSlice {
  // Workspace preview/dialog state, driven by dispatched runtime events.
  workspacePreviewDirty: boolean;
  workspacePreviewTransition: WorkspacePreviewTransitionState;
  workspaceDialogState: WorkspaceDialogState;

  dispatchWorkspaceRuntime: (event: WorkspaceRuntimeEvent) => WorkspaceRuntimeState;
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
