import type { MatchedDbEntry } from '../../../lib/bindings';
import type { ModFolder } from '../../../types/mod';
import type { DuplicateInfo } from '../../../types/scanner';
import type { WorkspaceDialogState } from '../../workspace-runtime/state/workspaceState';
import type { SyncCurrentData } from '../../workspace-runtime/state/workspaceState';
import { closeWorkspaceDialog } from '../../workspace-runtime/state/workspaceDialogs';
import { dispatchWorkspaceRuntimeEvent } from '../../workspace-runtime/state/workspaceStoreBridge';

export interface SharedModDialogState {
  moveDialog: { open: boolean; folder: ModFolder | null };
  renameDialog: { open: boolean; folder: ModFolder | null };
  deleteConfirm: { open: boolean; folder: ModFolder | null };
  pinSafeDialog: { open: boolean; folder: ModFolder | null };
  activeContextDialog: { open: boolean; folder: ModFolder | null; isProcessing: boolean };
  duplicateWarning: {
    open: boolean;
    folder: ModFolder | null;
    duplicates: DuplicateInfo[];
  };
  syncConfirm: {
    open: boolean;
    folder: ModFolder | null;
    match: MatchedDbEntry | null;
    isLoading: boolean;
    currentData: SyncCurrentData | null;
  };
}

const INITIAL_DIALOG_STATE: SharedModDialogState = {
  moveDialog: { open: false, folder: null },
  renameDialog: { open: false, folder: null },
  deleteConfirm: { open: false, folder: null },
  pinSafeDialog: { open: false, folder: null },
  activeContextDialog: { open: false, folder: null, isProcessing: false },
  duplicateWarning: { open: false, folder: null, duplicates: [] },
  syncConfirm: {
    open: false,
    folder: null,
    match: null,
    isLoading: false,
    currentData: null,
  },
};

export function selectSharedModDialogState(
  dialogState: WorkspaceDialogState,
): SharedModDialogState {
  if (dialogState.kind === 'modMove') {
    return { ...INITIAL_DIALOG_STATE, moveDialog: { open: true, folder: dialogState.folder } };
  }

  if (dialogState.kind === 'modRename') {
    return {
      ...INITIAL_DIALOG_STATE,
      renameDialog: { open: true, folder: dialogState.folder },
    };
  }

  if (dialogState.kind === 'modDelete') {
    return {
      ...INITIAL_DIALOG_STATE,
      deleteConfirm: { open: true, folder: dialogState.folder },
    };
  }

  if (dialogState.kind === 'modPinSafe') {
    return {
      ...INITIAL_DIALOG_STATE,
      pinSafeDialog: { open: true, folder: dialogState.folder },
    };
  }

  if (dialogState.kind === 'modActiveContext') {
    return {
      ...INITIAL_DIALOG_STATE,
      activeContextDialog: {
        open: true,
        folder: dialogState.folder,
        isProcessing: dialogState.isProcessing,
      },
    };
  }

  if (dialogState.kind === 'modDuplicateWarning') {
    return {
      ...INITIAL_DIALOG_STATE,
      duplicateWarning: {
        open: true,
        folder: dialogState.folder,
        duplicates: dialogState.duplicates,
      },
    };
  }

  if (dialogState.kind === 'modSync') {
    return {
      ...INITIAL_DIALOG_STATE,
      syncConfirm: {
        open: true,
        folder: dialogState.folder,
        match: dialogState.match,
        isLoading: dialogState.isLoading,
        currentData: dialogState.currentData,
      },
    };
  }

  return INITIAL_DIALOG_STATE;
}

export function openModMoveDialog(folder: ModFolder): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'modMove', folder },
  });
}

export function closeModMoveDialog(): void {
  closeWorkspaceDialog('modMove');
}

export function openModRenameDialog(folder: ModFolder): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'modRename', folder },
  });
}

export function closeModRenameDialog(): void {
  closeWorkspaceDialog('modRename');
}

export function openModDeleteDialog(folder: ModFolder): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'modDelete', folder },
  });
}

export function closeModDeleteDialog(): void {
  closeWorkspaceDialog('modDelete');
}

export function openModPinSafeDialog(folder: ModFolder): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'modPinSafe', folder },
  });
}

export function closeModPinSafeDialog(): void {
  closeWorkspaceDialog('modPinSafe');
}

export function openModActiveContextDialog(folder: ModFolder): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'modActiveContext', folder, isProcessing: false },
  });
}

export function updateModActiveContextDialog(folder: ModFolder, isProcessing: boolean): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_UPDATED',
    dialog: { kind: 'modActiveContext', folder, isProcessing },
  });
}

export function closeModActiveContextDialog(): void {
  closeWorkspaceDialog('modActiveContext');
}

export function openModDuplicateWarningDialog(
  folder: ModFolder,
  duplicates: DuplicateInfo[],
): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'modDuplicateWarning', folder, duplicates },
  });
}

export function closeModDuplicateWarningDialog(): void {
  closeWorkspaceDialog('modDuplicateWarning');
}

export function openModSyncDialog(folder: ModFolder, currentData: SyncCurrentData): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: {
      kind: 'modSync',
      folder,
      match: null,
      isLoading: true,
      currentData,
    },
  });
}

export function updateModSyncDialog(
  folder: ModFolder,
  currentData: SyncCurrentData | null,
  match: MatchedDbEntry | null,
  isLoading: boolean,
): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_UPDATED',
    dialog: {
      kind: 'modSync',
      folder,
      match,
      isLoading,
      currentData,
    },
  });
}

export function closeModSyncDialog(): void {
  closeWorkspaceDialog('modSync');
}
