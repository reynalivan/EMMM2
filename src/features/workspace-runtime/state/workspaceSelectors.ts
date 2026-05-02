import type { AppState } from '../../../stores/useAppStore';
import {
  INITIAL_WORKSPACE_DIALOG_STATE,
  INITIAL_WORKSPACE_PREVIEW_TRANSITION,
  type WorkspaceRuntimeState,
} from './workspaceState';

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
