import type { WorkspaceFileInUseDialogData, WorkspaceDialogState } from './workspaceState';
import { dispatchWorkspaceRuntimeEvent } from './workspaceStoreBridge';

export function openFolderConflictManagerDialog(groupId?: string): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'folderConflicts', ...(groupId ? { groupId } : {}) },
  });
}

export function openRenameConfirmationDialog(): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'renameConfirmations' },
  });
}

export function openWorkspaceSourceRecoveryDialog(): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'sourceRecovery' },
  });
}

export function openWorkspaceFileInUseDialog(data: WorkspaceFileInUseDialogData): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'fileInUse', data },
  });
}

export function closeWorkspaceDialog(kind?: WorkspaceDialogState['kind']): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_CLOSED',
    kind,
  });
}
