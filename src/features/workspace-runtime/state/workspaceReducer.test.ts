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
});
