import type { MatchedDbEntry } from '../../../lib/bindings';
import type { WorkspaceObjectNode } from '../../../types/workspace';

export interface ObjectSyncCurrentData {
  name: string;
  object_type: string;
  metadata: Record<string, unknown> | null;
  thumbnail_path: string | null;
}

export interface SyncConfirmState {
  open: boolean;
  objectId: string;
  objectName: string;
  itemType: 'object' | 'folder';
  match: MatchedDbEntry | null;
  isLoading: boolean;
  currentData: ObjectSyncCurrentData | null;
}

export interface SharedObjectActionState {
  editObject: WorkspaceObjectNode | null;
  deleteObjectDialog: { open: boolean; id: string; name: string };
  forceDeleteObjectDialog: { open: boolean; id: string; name: string; count: number };
  syncConfirm: SyncConfirmState;
}

export type SharedObjectAction =
  | { type: 'openEdit'; object: WorkspaceObjectNode }
  | { type: 'closeEdit' }
  | { type: 'openDelete'; id: string; name: string }
  | { type: 'closeDelete' }
  | { type: 'openForceDelete'; id: string; name: string; count: number }
  | { type: 'closeForceDelete' }
  | {
      type: 'openSync';
      objectId: string;
      objectName: string;
      currentData: ObjectSyncCurrentData;
    }
  | { type: 'setSyncMatch'; match: MatchedDbEntry | null; isLoading: boolean }
  | { type: 'closeSync' };

export const SYNC_CONFIRM_RESET: SyncConfirmState = {
  open: false,
  objectId: '',
  objectName: '',
  itemType: 'object',
  match: null,
  isLoading: false,
  currentData: null,
};

export const INITIAL_SHARED_OBJECT_ACTION_STATE: SharedObjectActionState = {
  editObject: null,
  deleteObjectDialog: { open: false, id: '', name: '' },
  forceDeleteObjectDialog: { open: false, id: '', name: '', count: 0 },
  syncConfirm: SYNC_CONFIRM_RESET,
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
    case 'openSync':
      return {
        ...state,
        syncConfirm: {
          open: true,
          objectId: action.objectId,
          objectName: action.objectName,
          itemType: 'object',
          match: null,
          isLoading: true,
          currentData: action.currentData,
        },
      };
    case 'setSyncMatch':
      return {
        ...state,
        syncConfirm: {
          ...state.syncConfirm,
          match: action.match,
          isLoading: action.isLoading,
        },
      };
    case 'closeSync':
      return {
        ...state,
        syncConfirm: SYNC_CONFIRM_RESET,
      };
    default:
      return state;
  }
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
