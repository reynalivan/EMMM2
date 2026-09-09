import type { WorkspaceObjectNode } from '@/entities/workspace';
import type { WorkspaceRuntimeEvent } from '../state/workspaceEvents';
import type { WorkspaceDialogState } from '../state/workspaceState';

export interface SharedObjectActionState {
  editObject: WorkspaceObjectNode | null;
  deleteObjectDialog: { open: boolean; id: string; name: string };
  forceDeleteObjectDialog: { open: boolean; id: string; name: string; count: number };
}

export type SharedObjectAction =
  | { type: 'openEdit'; object: WorkspaceObjectNode }
  | { type: 'closeEdit' }
  | { type: 'openDelete'; id: string; name: string }
  | { type: 'closeDelete' }
  | { type: 'openForceDelete'; id: string; name: string; count: number }
  | { type: 'closeForceDelete' };

export const INITIAL_SHARED_OBJECT_ACTION_STATE: SharedObjectActionState = {
  editObject: null,
  deleteObjectDialog: { open: false, id: '', name: '' },
  forceDeleteObjectDialog: { open: false, id: '', name: '', count: 0 },
};

export function sharedObjectActionsReducer(
  state: SharedObjectActionState,
  action: SharedObjectAction,
): SharedObjectActionState {
  switch (action.type) {
    case 'openEdit':
      return { ...state, editObject: action.object };
    case 'closeEdit':
      return { ...state, editObject: null };
    case 'openDelete':
      return {
        ...state,
        deleteObjectDialog: { open: true, id: action.id, name: action.name },
      };
    case 'closeDelete':
      return {
        ...state,
        deleteObjectDialog: INITIAL_SHARED_OBJECT_ACTION_STATE.deleteObjectDialog,
      };
    case 'openForceDelete':
      return {
        ...state,
        forceDeleteObjectDialog: {
          open: true,
          id: action.id,
          name: action.name,
          count: action.count,
        },
      };
    case 'closeForceDelete':
      return {
        ...state,
        forceDeleteObjectDialog: INITIAL_SHARED_OBJECT_ACTION_STATE.forceDeleteObjectDialog,
      };
    default:
      return state;
  }
}

/**
 * Projects the single workspace dialog slot back into the shared object action
 * state. Only one object dialog can be open at a time, so every branch starts
 * from the initial state.
 */
export function buildSharedObjectActionState(
  dialogState: WorkspaceDialogState,
): SharedObjectActionState {
  if (dialogState.kind === 'objectEdit') {
    return {
      ...INITIAL_SHARED_OBJECT_ACTION_STATE,
      editObject: dialogState.object,
    };
  }
  if (dialogState.kind === 'objectDelete') {
    return {
      ...INITIAL_SHARED_OBJECT_ACTION_STATE,
      deleteObjectDialog: {
        open: true,
        id: dialogState.id,
        name: dialogState.name,
      },
    };
  }
  if (dialogState.kind === 'objectForceDelete') {
    return {
      ...INITIAL_SHARED_OBJECT_ACTION_STATE,
      forceDeleteObjectDialog: {
        open: true,
        id: dialogState.id,
        name: dialogState.name,
        count: dialogState.count,
      },
    };
  }
  return INITIAL_SHARED_OBJECT_ACTION_STATE;
}

/**
 * Inverse of buildSharedObjectActionState: turns a reduced action state into the
 * single runtime event that keeps the workspace dialog slot in sync, or null
 * when nothing has to change.
 */
export function buildSharedObjectDialogEvent(
  reduced: SharedObjectActionState,
  currentDialogKind: WorkspaceDialogState['kind'],
): WorkspaceRuntimeEvent | null {
  if (reduced.editObject) {
    return {
      type: 'DIALOG_OPENED',
      dialog: { kind: 'objectEdit', object: reduced.editObject },
    };
  }

  if (reduced.deleteObjectDialog.open) {
    return {
      type: 'DIALOG_OPENED',
      dialog: {
        kind: 'objectDelete',
        id: reduced.deleteObjectDialog.id,
        name: reduced.deleteObjectDialog.name,
      },
    };
  }

  if (reduced.forceDeleteObjectDialog.open) {
    return {
      type: 'DIALOG_OPENED',
      dialog: {
        kind: 'objectForceDelete',
        id: reduced.forceDeleteObjectDialog.id,
        name: reduced.forceDeleteObjectDialog.name,
        count: reduced.forceDeleteObjectDialog.count,
      },
    };
  }

  if (currentDialogKind.startsWith('object')) {
    return { type: 'DIALOG_CLOSED', kind: currentDialogKind };
  }

  return null;
}

/** Message interpolated into the object mutation error toast. */
export function describeObjectMutationError(error: unknown): string {
  return String((error as Record<string, unknown>)?.message ?? error);
}

export function parseObjectHasModsError(error: unknown): number | null {
  const errorString = String((error as Record<string, unknown>)?.message ?? error);

  try {
    const payload = typeof error === 'string' ? JSON.parse(error) : error;
    if (payload && typeof payload === 'object' && 'ObjectHasMods' in payload) {
      return Number((payload as Record<string, unknown>).ObjectHasMods);
    }
  } catch {
    // Fall back to string parsing below.
  }

  if (!errorString.includes('ObjectHasMods') && !errorString.includes('Object has')) {
    return null;
  }

  const match = errorString.match(/\d+/);
  if (!match) {
    return 1;
  }

  return parseInt(match[0], 10);
}
