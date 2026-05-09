import { describe, expect, it } from 'vitest';
import { reduceWorkspaceRuntimeState } from './workspaceReducer';
import type { WorkspaceRuntimeState } from './workspaceState';

const baseState: WorkspaceRuntimeState = {
  selectedObjectFolderPath: null,
  explorerSubPath: undefined,
  currentPath: [],
  selectedModPath: null,
  mobileActivePane: 'sidebar',
  previewDirty: false,
  previewTransition: { kind: 'idle', pendingTarget: null },
  dialogState: { kind: 'none' },
};

describe('workspaceReducer', () => {
  it('focuses object and resets explorer selection', () => {
    const nextState = reduceWorkspaceRuntimeState(baseState, {
      type: 'OBJECT_FOCUSED',
      folderPath: 'ALBEDO',
    });

    expect(nextState.selectedObjectFolderPath).toBe('ALBEDO');
    expect(nextState.explorerSubPath).toBe('ALBEDO');
    expect(nextState.currentPath).toEqual(['ALBEDO']);
    expect(nextState.selectedModPath).toBeNull();
    expect(nextState.mobileActivePane).toBe('grid');
  });

  it('queues preview transition instead of changing selection while dirty', () => {
    const dirtyState: WorkspaceRuntimeState = {
      ...baseState,
      selectedModPath: 'E:/Mods/ALBEDO/mod.ini',
      previewDirty: true,
    };

    const nextState = reduceWorkspaceRuntimeState(dirtyState, {
      type: 'MOD_SELECTED',
      path: 'E:/Mods/ALBEDO/variant.ini',
      mobilePane: 'details',
    });

    expect(nextState.selectedModPath).toBe('E:/Mods/ALBEDO/mod.ini');
    expect(nextState.previewTransition.kind).toBe('pending');
    expect(nextState.dialogState.kind).toBe('previewUnsavedChanges');
  });

  it('confirms pending transition and applies selected mod', () => {
    const pendingState: WorkspaceRuntimeState = {
      ...baseState,
      selectedModPath: 'E:/Mods/ALBEDO/mod.ini',
      previewDirty: true,
      previewTransition: {
        kind: 'pending',
        pendingTarget: {
          kind: 'selectMod',
          path: 'E:/Mods/ALBEDO/variant.ini',
          mobilePane: 'details',
        },
      },
      dialogState: { kind: 'previewUnsavedChanges' },
    };

    const nextState = reduceWorkspaceRuntimeState(pendingState, {
      type: 'PREVIEW_TRANSITION_CONFIRMED',
    });

    expect(nextState.selectedModPath).toBe('E:/Mods/ALBEDO/variant.ini');
    expect(nextState.previewDirty).toBe(false);
    expect(nextState.previewTransition.kind).toBe('idle');
    expect(nextState.dialogState.kind).toBe('none');
  });

  it('rewrites relative explorer path and absolute selected mod path together', () => {
    const nextState = reduceWorkspaceRuntimeState(
      {
        ...baseState,
        selectedObjectFolderPath: 'AMBERCN',
        explorerSubPath: 'AMBERCN/Variants',
        currentPath: ['AMBERCN', 'Variants'],
        selectedModPath: 'E:/Mods/AMBERCN/Variants/School/mod.ini',
      },
      {
        type: 'PATHS_REWRITTEN',
        rewrites: [
          {
            oldPath: 'E:/Mods/AMBERCN/Variants',
            newPath: 'E:/Mods/AMBERCN/Presets',
          },
        ],
      },
    );

    expect(nextState.explorerSubPath).toBe('AMBERCN/Presets');
    expect(nextState.currentPath).toEqual(['AMBERCN', 'Presets']);
    expect(nextState.selectedModPath).toBe('E:/Mods/AMBERCN/Presets/School/mod.ini');
  });

  it('reconciles stale runtime selection from the workspace read model', () => {
    const nextState = reduceWorkspaceRuntimeState(
      {
        ...baseState,
        selectedObjectFolderPath: 'STALE_OBJECT',
        explorerSubPath: 'STALE_OBJECT/Deleted',
        currentPath: ['STALE_OBJECT', 'Deleted'],
        selectedModPath: 'E:/Mods/STALE_OBJECT/Deleted',
        previewDirty: true,
        previewTransition: {
          kind: 'pending',
          pendingTarget: { kind: 'selectMod', path: 'E:/Mods/Other' },
        },
        dialogState: { kind: 'previewUnsavedChanges' },
      },
      {
        type: 'SELECTION_RECONCILED',
        selectedObjectFolderPath: null,
        explorerSubPath: undefined,
        selectedModPath: null,
        currentPath: [],
        reconciliationStatus: 'cleared',
        reconciliationReason: 'missing_object_root',
        affectedPaths: ['STALE_OBJECT'],
      },
    );

    expect(nextState.selectedObjectFolderPath).toBeNull();
    expect(nextState.explorerSubPath).toBeUndefined();
    expect(nextState.currentPath).toEqual([]);
    expect(nextState.selectedModPath).toBeNull();
    expect(nextState.previewDirty).toBe(false);
    expect(nextState.previewTransition.kind).toBe('idle');
    expect(nextState.dialogState.kind).toBe('none');
  });

  it('clears dirty preview when disk invalidates the selected target', () => {
    const nextState = reduceWorkspaceRuntimeState(
      {
        ...baseState,
        selectedObjectFolderPath: 'ALBEDO',
        explorerSubPath: 'ALBEDO',
        currentPath: ['ALBEDO'],
        selectedModPath: 'E:/Mods/ALBEDO/Deleted',
        previewDirty: true,
        dialogState: { kind: 'previewUnsavedChanges' },
      },
      {
        type: 'TARGETS_INVALIDATED',
        paths: ['E:/Mods/ALBEDO/Deleted'],
        resetExplorer: true,
      },
    );

    expect(nextState.selectedModPath).toBeNull();
    expect(nextState.previewDirty).toBe(false);
    expect(nextState.previewTransition.kind).toBe('idle');
    expect(nextState.dialogState.kind).toBe('none');
  });
});
