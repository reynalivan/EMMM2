import { describe, expect, it } from 'vitest';
import type { WorkspaceObjectNode } from '../../../types/workspace';
import {
  buildSharedObjectActionState,
  buildSharedObjectDialogEvent,
  describeObjectMutationError,
  INITIAL_SHARED_OBJECT_ACTION_STATE,
  parseObjectHasModsError,
  sharedObjectActionsReducer,
} from './sharedObjectActionsState';

const object = { id: 'object-1', name: 'Alpha' } as unknown as WorkspaceObjectNode;

describe('buildSharedObjectActionState', () => {
  it('returns the initial state for non-object dialogs', () => {
    expect(buildSharedObjectActionState({ kind: 'none' })).toBe(INITIAL_SHARED_OBJECT_ACTION_STATE);
    expect(buildSharedObjectActionState({ kind: 'previewUnsavedChanges' })).toBe(
      INITIAL_SHARED_OBJECT_ACTION_STATE,
    );
  });

  it('projects each object dialog into exactly one open slot', () => {
    expect(buildSharedObjectActionState({ kind: 'objectEdit', object })).toEqual({
      ...INITIAL_SHARED_OBJECT_ACTION_STATE,
      editObject: object,
    });

    expect(
      buildSharedObjectActionState({ kind: 'objectDelete', id: 'object-1', name: 'Alpha' }),
    ).toEqual({
      ...INITIAL_SHARED_OBJECT_ACTION_STATE,
      deleteObjectDialog: { open: true, id: 'object-1', name: 'Alpha' },
    });

    expect(
      buildSharedObjectActionState({
        kind: 'objectForceDelete',
        id: 'object-1',
        name: 'Alpha',
        count: 3,
      }),
    ).toEqual({
      ...INITIAL_SHARED_OBJECT_ACTION_STATE,
      forceDeleteObjectDialog: { open: true, id: 'object-1', name: 'Alpha', count: 3 },
    });

    expect(
      buildSharedObjectActionState({
        kind: 'objectSync',
        objectId: 'object-1',
        objectName: 'Alpha',
        itemType: 'object',
        match: null,
        isLoading: true,
        currentData: null,
      }).syncConfirm,
    ).toEqual({
      open: true,
      objectId: 'object-1',
      objectName: 'Alpha',
      itemType: 'object',
      match: null,
      isLoading: true,
      currentData: null,
    });
  });
});

describe('buildSharedObjectDialogEvent', () => {
  it('opens the edit dialog', () => {
    const reduced = sharedObjectActionsReducer(INITIAL_SHARED_OBJECT_ACTION_STATE, {
      type: 'openEdit',
      object,
    });

    expect(buildSharedObjectDialogEvent(reduced, 'none')).toEqual({
      type: 'DIALOG_OPENED',
      dialog: { kind: 'objectEdit', object },
    });
  });

  it('escalates delete into force delete', () => {
    const reduced = sharedObjectActionsReducer(INITIAL_SHARED_OBJECT_ACTION_STATE, {
      type: 'openForceDelete',
      id: 'object-1',
      name: 'Alpha',
      count: 2,
    });

    expect(buildSharedObjectDialogEvent(reduced, 'objectDelete')).toEqual({
      type: 'DIALOG_OPENED',
      dialog: { kind: 'objectForceDelete', id: 'object-1', name: 'Alpha', count: 2 },
    });
  });

  it('updates instead of reopening an already open sync dialog', () => {
    const reduced = sharedObjectActionsReducer(INITIAL_SHARED_OBJECT_ACTION_STATE, {
      type: 'openSync',
      objectId: 'object-1',
      objectName: 'Alpha',
      currentData: {
        name: 'Alpha',
        object_type: 'Character',
        metadata: null,
        thumbnail_path: null,
      },
    });

    expect(buildSharedObjectDialogEvent(reduced, 'objectSync')).toMatchObject({
      type: 'DIALOG_UPDATED',
    });
    expect(buildSharedObjectDialogEvent(reduced, 'none')).toMatchObject({
      type: 'DIALOG_OPENED',
    });
  });

  it('closes only object-owned dialogs when nothing is open', () => {
    expect(buildSharedObjectDialogEvent(INITIAL_SHARED_OBJECT_ACTION_STATE, 'objectEdit')).toEqual({
      type: 'DIALOG_CLOSED',
      kind: 'objectEdit',
    });
    expect(
      buildSharedObjectDialogEvent(INITIAL_SHARED_OBJECT_ACTION_STATE, 'modRename'),
    ).toBeNull();
    expect(buildSharedObjectDialogEvent(INITIAL_SHARED_OBJECT_ACTION_STATE, 'none')).toBeNull();
  });
});

describe('object mutation error helpers', () => {
  it('prefers the error message field', () => {
    expect(describeObjectMutationError(new Error('boom'))).toBe('boom');
    expect(describeObjectMutationError('plain')).toBe('plain');
  });

  it('extracts the blocking mod count', () => {
    expect(parseObjectHasModsError({ ObjectHasMods: 4 })).toBe(4);
    expect(parseObjectHasModsError(new Error('ObjectHasMods 3'))).toBe(3);
    expect(parseObjectHasModsError(new Error('unrelated'))).toBeNull();
  });
});
