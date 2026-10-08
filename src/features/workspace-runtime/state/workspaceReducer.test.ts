import { describe, expect, it, vi } from 'vitest';
import { reduceWorkspaceRuntimeState } from './workspaceRuntimeReducer';
import type { WorkspaceRuntimeState } from './workspaceState';

const baseState: WorkspaceRuntimeState = {
  selectedObjectFolderPath: null,
  explorerSubPath: undefined,
  currentPath: [],
  selectedModPath: null,
  gridSelection: new Set(),
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

  it('confirms a pending grid selection effect atomically with the selected mod', () => {
    const pendingState: WorkspaceRuntimeState = {
      ...baseState,
      selectedModPath: 'E:/Mods/ALBEDO/mod.ini',
      gridSelection: new Set(['E:/Mods/ALBEDO/mod.ini']),
      previewDirty: true,
      previewTransition: {
        kind: 'pending',
        pendingTarget: {
          kind: 'selectMod',
          path: 'E:/Mods/ALBEDO/variant.ini',
          selectionEffect: { gridSelection: ['E:/Mods/ALBEDO/variant.ini'] },
        },
      },
      dialogState: { kind: 'previewUnsavedChanges' },
    };

    const nextState = reduceWorkspaceRuntimeState(pendingState, {
      type: 'PREVIEW_TRANSITION_CONFIRMED',
    });

    expect(nextState.selectedModPath).toBe('E:/Mods/ALBEDO/variant.ini');
    expect(nextState.gridSelection).toEqual(new Set(['E:/Mods/ALBEDO/variant.ini']));
  });

  it('rewrites a pending selection target and frozen grid paths before confirmation', () => {
    const pendingState: WorkspaceRuntimeState = {
      ...baseState,
      selectedModPath: 'E:/Mods/ALBEDO/mod.ini',
      gridSelection: new Set(['E:/Mods/ALBEDO/mod.ini']),
      previewDirty: true,
      previewTransition: {
        kind: 'pending',
        pendingTarget: {
          kind: 'selectMod',
          path: 'E:/Mods/ALBEDO/variant.ini',
          selectionEffect: { gridSelection: ['E:/Mods/ALBEDO/variant.ini'] },
        },
      },
      dialogState: { kind: 'previewUnsavedChanges' },
    };
    const rewrittenState = reduceWorkspaceRuntimeState(pendingState, {
      type: 'PATHS_REWRITTEN',
      rewrites: [{ oldPath: 'E:/Mods/ALBEDO/variant.ini', newPath: 'E:/Mods/ALBEDO/renamed.ini' }],
    });

    expect(rewrittenState.selectedModPath).toBe('E:/Mods/ALBEDO/mod.ini');
    expect(rewrittenState.previewTransition).toMatchObject({
      kind: 'pending',
      pendingTarget: {
        kind: 'selectMod',
        path: 'E:/Mods/ALBEDO/renamed.ini',
        selectionEffect: { gridSelection: ['E:/Mods/ALBEDO/renamed.ini'] },
      },
    });

    const confirmedState = reduceWorkspaceRuntimeState(rewrittenState, {
      type: 'PREVIEW_TRANSITION_CONFIRMED',
    });
    expect(confirmedState.selectedModPath).toBe('E:/Mods/ALBEDO/renamed.ini');
    expect(confirmedState.gridSelection).toEqual(new Set(['E:/Mods/ALBEDO/renamed.ini']));
  });

  it('rewrites a pending object-focus target before confirmation', () => {
    const pendingState: WorkspaceRuntimeState = {
      ...baseState,
      selectedModPath: 'E:/Mods/ALBEDO/mod.ini',
      previewDirty: true,
      previewTransition: {
        kind: 'pending',
        pendingTarget: { kind: 'focusObject', folderPath: 'E:/Objects/ALBEDO' },
      },
      dialogState: { kind: 'previewUnsavedChanges' },
    };

    const rewrittenState = reduceWorkspaceRuntimeState(pendingState, {
      type: 'PATHS_REWRITTEN',
      rewrites: [{ oldPath: 'E:/Objects/ALBEDO', newPath: 'E:/Objects/ALBEDO-Renamed' }],
    });
    expect(rewrittenState.previewTransition).toMatchObject({
      kind: 'pending',
      pendingTarget: { kind: 'focusObject', folderPath: 'E:/Objects/ALBEDO-Renamed' },
    });

    const confirmedState = reduceWorkspaceRuntimeState(rewrittenState, {
      type: 'PREVIEW_TRANSITION_CONFIRMED',
    });
    expect(confirmedState.selectedObjectFolderPath).toBe('E:/Objects/ALBEDO-Renamed');
    expect(confirmedState.explorerSubPath).toBe('E:/Objects/ALBEDO-Renamed');
  });

  it('rewrites a pending explorer target and rebuilds its current path', () => {
    const pendingState: WorkspaceRuntimeState = {
      ...baseState,
      selectedObjectFolderPath: 'E:/Objects/ALBEDO',
      explorerSubPath: 'E:/Objects/ALBEDO',
      currentPath: ['ALBEDO'],
      selectedModPath: 'E:/Mods/ALBEDO/mod.ini',
      previewDirty: true,
      previewTransition: {
        kind: 'pending',
        pendingTarget: {
          kind: 'navigateExplorer',
          explorerSubPath: 'E:/Objects/ALBEDO/child',
          currentPath: ['ALBEDO', 'child'],
        },
      },
      dialogState: { kind: 'previewUnsavedChanges' },
    };

    const rewrittenState = reduceWorkspaceRuntimeState(pendingState, {
      type: 'PATHS_REWRITTEN',
      rewrites: [{ oldPath: 'E:/Objects/ALBEDO', newPath: 'E:/Objects/ALBEDO-Renamed' }],
    });
    expect(rewrittenState.previewTransition).toMatchObject({
      kind: 'pending',
      pendingTarget: {
        kind: 'navigateExplorer',
        explorerSubPath: 'E:/Objects/ALBEDO-Renamed/child',
        currentPath: ['ALBEDO-Renamed', 'child'],
      },
    });

    const confirmedState = reduceWorkspaceRuntimeState(rewrittenState, {
      type: 'PREVIEW_TRANSITION_CONFIRMED',
    });
    expect(confirmedState.explorerSubPath).toBe('E:/Objects/ALBEDO-Renamed/child');
    expect(confirmedState.currentPath).toEqual(['ALBEDO-Renamed', 'child']);
  });

  it('cancels an opaque all-matching completion when a captured path is rewritten', () => {
    const onApplied = vi.fn();
    const pendingState: WorkspaceRuntimeState = {
      ...baseState,
      selectedModPath: 'E:/Mods/ALBEDO/mod.ini',
      previewDirty: true,
      previewTransition: {
        kind: 'pending',
        pendingTarget: {
          kind: 'selectMod',
          path: 'E:/Mods/ALBEDO/variant.ini',
          selectionEffect: {
            gridSelection: [],
            affectedPaths: ['E:/Mods/ALBEDO/excluded.ini'],
            onApplied,
          },
        },
      },
      dialogState: { kind: 'previewUnsavedChanges' },
    };

    const rewrittenState = reduceWorkspaceRuntimeState(pendingState, {
      type: 'PATHS_REWRITTEN',
      rewrites: [{ oldPath: 'E:/Mods/ALBEDO/excluded.ini', newPath: 'E:/Mods/ALBEDO/renamed.ini' }],
    });

    expect(rewrittenState.previewTransition.kind).toBe('idle');
    expect(rewrittenState.dialogState.kind).toBe('none');
    expect(onApplied).not.toHaveBeenCalled();
  });

  it('preserves pending preview confirmation for an unchanged reconciled selection', () => {
    const pendingState: WorkspaceRuntimeState = {
      ...baseState,
      selectedObjectFolderPath: 'ALBEDO',
      explorerSubPath: 'ALBEDO',
      currentPath: ['ALBEDO'],
      selectedModPath: 'E:/Mods/ALBEDO/mod.ini',
      previewDirty: true,
      previewTransition: {
        kind: 'pending',
        pendingTarget: { kind: 'selectMod', path: 'E:/Mods/ALBEDO/variant.ini' },
      },
      dialogState: { kind: 'previewUnsavedChanges' },
    };

    const reconciledState = reduceWorkspaceRuntimeState(pendingState, {
      type: 'SELECTION_RECONCILED',
      selectedObjectFolderPath: 'ALBEDO',
      explorerSubPath: 'ALBEDO',
      selectedModPath: 'E:/Mods/ALBEDO/mod.ini',
      currentPath: ['ALBEDO'],
      reconciliationStatus: 'unchanged',
      reconciliationReason: null,
      affectedPaths: [],
    });

    expect(reconciledState.selectedModPath).toBe('E:/Mods/ALBEDO/mod.ini');
    expect(reconciledState.previewDirty).toBe(true);
    expect(reconciledState.previewTransition.kind).toBe('pending');
    expect(reconciledState.dialogState.kind).toBe('previewUnsavedChanges');

    const confirmedState = reduceWorkspaceRuntimeState(reconciledState, {
      type: 'PREVIEW_TRANSITION_CONFIRMED',
    });
    expect(confirmedState.selectedModPath).toBe('E:/Mods/ALBEDO/variant.ini');
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

  it('invalidates selection whose case differs from the invalidated path (Windows)', () => {
    const nextState = reduceWorkspaceRuntimeState(
      {
        ...baseState,
        selectedObjectFolderPath: 'E:/Mods/ALBEDO',
        explorerSubPath: 'E:/Mods/ALBEDO',
        currentPath: ['ALBEDO'],
        selectedModPath: 'E:/Mods/ALBEDO/Deleted',
        previewDirty: true,
        dialogState: { kind: 'previewUnsavedChanges' },
      },
      {
        type: 'TARGETS_INVALIDATED',
        paths: ['e:\\mods\\albedo'],
        resetExplorer: true,
      },
    );

    expect(nextState.selectedObjectFolderPath).toBeNull();
    expect(nextState.selectedModPath).toBeNull();
    expect(nextState.explorerSubPath).toBeUndefined();
    expect(nextState.currentPath).toEqual([]);
    expect(nextState.previewDirty).toBe(false);
    expect(nextState.dialogState.kind).toBe('none');
  });

  it('clears dirty preview when source unavailable reconciliation removes selected target', () => {
    const nextState = reduceWorkspaceRuntimeState(
      {
        ...baseState,
        selectedObjectFolderPath: 'ALBEDO',
        explorerSubPath: 'ALBEDO',
        currentPath: ['ALBEDO'],
        selectedModPath: 'E:/Mods/ALBEDO',
        previewDirty: true,
        dialogState: { kind: 'previewUnsavedChanges' },
      },
      {
        type: 'SELECTION_RECONCILED',
        selectedObjectFolderPath: null,
        explorerSubPath: undefined,
        selectedModPath: null,
        currentPath: [],
        reconciliationStatus: 'cleared',
        reconciliationReason: 'source_unavailable',
        affectedPaths: ['E:/Mods'],
      },
    );

    expect(nextState.selectedModPath).toBeNull();
    expect(nextState.previewDirty).toBe(false);
    expect(nextState.previewTransition.kind).toBe('idle');
    expect(nextState.dialogState.kind).toBe('none');
  });

  it('preserves dirty preview when reconciled paths do not touch selected target', () => {
    const nextState = reduceWorkspaceRuntimeState(
      {
        ...baseState,
        selectedObjectFolderPath: 'ALBEDO',
        explorerSubPath: 'ALBEDO',
        currentPath: ['ALBEDO'],
        selectedModPath: 'E:/Mods/ALBEDO',
        previewDirty: true,
      },
      {
        type: 'SELECTION_RECONCILED',
        selectedObjectFolderPath: 'ALBEDO',
        explorerSubPath: 'ALBEDO',
        selectedModPath: 'E:/Mods/ALBEDO',
        currentPath: ['ALBEDO'],
        reconciliationStatus: 'fallback',
        reconciliationReason: 'missing_explorer_path',
        affectedPaths: ['E:/Mods/AMBER'],
      },
    );

    expect(nextState.selectedModPath).toBe('E:/Mods/ALBEDO');
    expect(nextState.previewDirty).toBe(true);
  });
});
