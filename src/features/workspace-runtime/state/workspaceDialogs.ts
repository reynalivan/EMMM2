import type { WorkspaceExplorerNode } from '../../../types/workspace';
import type {
  WorkspaceFileInUseDialogData,
  WorkspaceDialogState,
  WorkspaceRenameConflict,
} from './workspaceState';
import { dispatchWorkspaceRuntimeEvent } from './workspaceStoreBridge';

export function openWorkspaceConflictDialog(conflict: WorkspaceRenameConflict): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'conflict', conflict },
  });
}

export function openWorkspaceFileInUseDialog(data: WorkspaceFileInUseDialogData): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'fileInUse', data },
  });
}

export function openWorkspaceEnableParentDialog(data: {
  ancestorName: string;
  ancestorPath: string;
  willActivate: WorkspaceExplorerNode[];
  stayDisabled: WorkspaceExplorerNode[];
}): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind: 'folderEnableParent', ...data },
  });
}

export function closeWorkspaceDialog(kind?: WorkspaceDialogState['kind']): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_CLOSED',
    kind,
  });
}
