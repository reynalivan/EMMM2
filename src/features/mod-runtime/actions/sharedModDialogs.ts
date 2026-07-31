import type { MatchedDbEntry } from '../../../lib/bindings';
import type { ModFolder } from '../../../types/object';
import type { DuplicateInfo } from '../../../types/scanner';
import type { WorkspaceDialogState } from '../../workspace-runtime/state/workspaceState';
import type { SyncCurrentData } from '../../workspace-runtime/state/workspaceState';
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

const DIALOG_FIELD = {
  modMove: 'moveDialog',
  modRename: 'renameDialog',
  modDelete: 'deleteConfirm',
  modPinSafe: 'pinSafeDialog',
  modActiveContext: 'activeContextDialog',
  modDuplicateWarning: 'duplicateWarning',
  modSync: 'syncConfirm',
} as const satisfies Record<string, keyof SharedModDialogState>;

export type ModDialogKind = keyof typeof DIALOG_FIELD;
type ModDialog<K extends ModDialogKind> = Extract<WorkspaceDialogState, { kind: K }>;
type ModDialogPayload<K extends ModDialogKind> = Omit<ModDialog<K>, 'kind'>;

/**
 * Flatten the active dialog into the seven boolean-flag slices the mod surfaces
 * render from. Every dialog payload is field-compatible with its slice, so the
 * spread is a straight merge.
 */
export function selectSharedModDialogState(
  dialogState: WorkspaceDialogState,
): SharedModDialogState {
  const field: keyof SharedModDialogState | undefined =
    DIALOG_FIELD[dialogState.kind as ModDialogKind];
  if (!field) {
    return INITIAL_DIALOG_STATE;
  }

  const { kind: _kind, ...payload } = dialogState;
  // ponytail: computed-key spreads always widen, so one assertion here beats
  // seven hand-written branches TypeScript could not check any better.
  return {
    ...INITIAL_DIALOG_STATE,
    [field]: { ...INITIAL_DIALOG_STATE[field], ...payload, open: true },
  } as SharedModDialogState;
}

export function openModDialog<K extends ModDialogKind>(
  kind: K,
  payload: ModDialogPayload<K>,
): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_OPENED',
    dialog: { kind, ...payload } as ModDialog<K>,
  });
}

export function updateModDialog<K extends ModDialogKind>(
  kind: K,
  payload: ModDialogPayload<K>,
): void {
  dispatchWorkspaceRuntimeEvent({
    type: 'DIALOG_UPDATED',
    dialog: { kind, ...payload } as ModDialog<K>,
  });
}
