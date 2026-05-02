import type {
  WorkspaceDialogState,
  WorkspaceMobilePane,
  WorkspaceTransitionTarget,
} from './workspaceState';

export type WorkspaceRuntimeEvent =
  | { type: 'OBJECT_FOCUSED'; folderPath: string }
  | { type: 'OBJECT_CLEARED'; resetExplorer: boolean; mobilePane?: WorkspaceMobilePane }
  | {
      type: 'EXPLORER_NAVIGATED';
      currentPath: string[];
      explorerSubPath?: string;
    }
  | { type: 'MOD_SELECTED'; path: string | null; mobilePane?: WorkspaceMobilePane }
  | {
      type: 'SELECTION_CLEARED';
      resetExplorer: boolean;
      mobilePane?: WorkspaceMobilePane;
      clearObjectSelection?: boolean;
      force?: boolean;
    }
  | {
      type: 'PATHS_REWRITTEN';
      rewrites: Array<{ oldPath: string; newPath: string }>;
    }
  | { type: 'TARGETS_INVALIDATED'; paths: string[]; resetExplorer?: boolean }
  | { type: 'PREVIEW_DIRTY_CHANGED'; dirty: boolean }
  | { type: 'PREVIEW_TRANSITION_REQUESTED'; target: WorkspaceTransitionTarget }
  | { type: 'PREVIEW_TRANSITION_CONFIRMED' }
  | { type: 'PREVIEW_TRANSITION_CANCELLED' }
  | { type: 'DIALOG_OPENED'; dialog: Exclude<WorkspaceDialogState, { kind: 'none' }> }
  | { type: 'DIALOG_UPDATED'; dialog: Exclude<WorkspaceDialogState, { kind: 'none' }> }
  | { type: 'DIALOG_CLOSED'; kind?: WorkspaceDialogState['kind'] };
