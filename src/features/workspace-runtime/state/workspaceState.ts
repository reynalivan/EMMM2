import type { ModFolder } from '@/entities/game-object';
import type { DuplicateInfo } from '@/entities/workspace';
import type {
  WorkspaceObjectNode,
  WorkspaceParentEnableRequirement,
  WorkspaceSwitchInput,
} from '@/entities/workspace';

export type WorkspaceMobilePane = 'sidebar' | 'grid' | 'details';
export type WorkspaceDuplicateTarget = Pick<ModFolder, 'id' | 'path' | 'name'>;

export interface WorkspaceFileInUseDialogData {
  path: string;
  processes: string[];
  onRetry?: () => void;
}

export type WorkspaceDialogState =
  | { kind: 'none' }
  | { kind: 'previewUnsavedChanges' }
  | { kind: 'folderConflicts' }
  | { kind: 'renameConfirmations' }
  | { kind: 'sourceRecovery' }
  | { kind: 'fileInUse'; data: WorkspaceFileInUseDialogData }
  | { kind: 'modMove'; folder: ModFolder }
  | { kind: 'modRename'; folder: ModFolder }
  | { kind: 'modDelete'; folder: ModFolder }
  | { kind: 'modActiveContext'; folder: ModFolder; isProcessing: boolean }
  | {
      kind: 'modDuplicateWarning';
      folder: WorkspaceDuplicateTarget;
      duplicates: DuplicateInfo[];
      requiresResolution: boolean;
      enableDisabledAncestors: boolean;
      parentEnableConfirmation: string | null;
    }
  | {
      kind: 'folderEnableParent';
      folder: WorkspaceDuplicateTarget;
      requirement: WorkspaceParentEnableRequirement;
      resumeInput: WorkspaceSwitchInput;
    }
  | { kind: 'objectEdit'; object: WorkspaceObjectNode }
  | { kind: 'objectDelete'; id: string; name: string }
  | { kind: 'objectForceDelete'; id: string; name: string; count: number };

export type WorkspaceTransitionTarget =
  | { kind: 'focusObject'; folderPath: string }
  | { kind: 'navigateExplorer'; currentPath: string[]; explorerSubPath?: string }
  | { kind: 'selectMod'; path: string | null; mobilePane?: WorkspaceMobilePane }
  | { kind: 'collapseSection'; sectionId: string }
  | {
      kind: 'clearSelection';
      resetExplorer: boolean;
      mobilePane?: WorkspaceMobilePane;
      clearObjectSelection?: boolean;
    };

export type WorkspacePreviewTransitionState =
  | { kind: 'idle'; pendingTarget: null }
  | { kind: 'pending'; pendingTarget: WorkspaceTransitionTarget };

export interface WorkspaceRuntimeState {
  selectedObjectFolderPath: string | null;
  explorerSubPath: string | undefined;
  currentPath: string[];
  selectedModPath: string | null;
  mobileActivePane: WorkspaceMobilePane;
  previewDirty: boolean;
  previewTransition: WorkspacePreviewTransitionState;
  dialogState: WorkspaceDialogState;
}

export const INITIAL_WORKSPACE_DIALOG_STATE: WorkspaceDialogState = { kind: 'none' };
export const INITIAL_WORKSPACE_PREVIEW_TRANSITION: WorkspacePreviewTransitionState = {
  kind: 'idle',
  pendingTarget: null,
};
