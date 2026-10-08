import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useFolderGrid } from './useFolderGrid';
import { useAppStore } from '@/app/store';
import { createWrapper } from '../../../tests/testing/test-utils';
import { ModFolder } from '@/entities/game-object';

// Provide element dimensions for virtualization
globalThis.ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
};

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));

vi.mock('./folderCache', () => ({
  sortFolders: (f: ModFolder[]) => f,
  folderKeys: { all: [] },
}));

vi.mock('./useFolderMutations', () => ({
  useImportMods: () => ({ mutate: vi.fn() }),
  useToggleModSafe: () => ({ mutate: vi.fn() }),
  useUpdateModInfo: () => ({ mutate: vi.fn() }),
  useActiveConflicts: () => ({ data: [] }),
}));

vi.mock('@/features/mod-runtime/hooks/useBulkModMutations', () => ({
  useBulkToggle: () => ({ mutate: vi.fn() }),
  useBulkDelete: () => ({ mutate: vi.fn() }),
  useBulkUpdateInfo: () => ({ mutate: vi.fn() }),
  useBulkSafety: () => ({ mutate: vi.fn() }),
  useBulkFavorite: () => ({ mutate: vi.fn() }),
  useBulkPin: () => ({ mutate: vi.fn() }),
}));

const { mockUseWorkspaceViewModel, mockUseWorkspaceExplorerPages, mockListingRevision } =
  vi.hoisted(() => ({
    mockUseWorkspaceExplorerPages: vi.fn(),
    mockListingRevision: { current: null as string | null },
    mockUseWorkspaceViewModel: vi.fn((_options?: unknown) => ({
      data: {
        explorer: {
          children: [
            {
              name: 'Mod A',
              display_name: 'Mod A',
              path: '/Mod A',
              folder_name: 'Mod A',
              node_type: 'ContainerFolder',
              node_kind: 'container',
              display_mode: 'container_folder',
              type_chip: null,
              is_enabled: true,
              is_effectively_active: true,
              ancestor_disabled: false,
              inactive_reason: null,
              warning_state: 'none',
              primary_warning: null,
              can_navigate: true,
            },
            {
              name: 'Mod B',
              display_name: 'Mod B',
              path: '/Mod B',
              folder_name: 'Mod B',
              node_type: 'ContainerFolder',
              node_kind: 'container',
              display_mode: 'container_folder',
              type_chip: null,
              is_enabled: true,
              is_effectively_active: true,
              ancestor_disabled: false,
              inactive_reason: null,
              warning_state: 'none',
              primary_warning: null,
              can_navigate: true,
            },
            {
              name: 'Mod C',
              display_name: 'Mod C',
              path: '/Mod C',
              folder_name: 'Mod C',
              node_type: 'ContainerFolder',
              node_kind: 'container',
              display_mode: 'container_folder',
              type_chip: null,
              is_enabled: true,
              is_effectively_active: true,
              ancestor_disabled: false,
              inactive_reason: null,
              warning_state: 'none',
              primary_warning: null,
              can_navigate: true,
            },
            {
              name: 'Mod D',
              display_name: 'Mod D',
              path: '/Mod D',
              folder_name: 'Mod D',
              node_type: 'ContainerFolder',
              node_kind: 'container',
              display_mode: 'container_folder',
              type_chip: null,
              is_enabled: true,
              is_effectively_active: true,
              ancestor_disabled: false,
              inactive_reason: null,
              warning_state: 'none',
              primary_warning: null,
              can_navigate: true,
            },
          ],
          self_node_type: null,
          self_node_kind: 'container',
          self_display_mode: 'unknown',
          self_type_chip: null,
          self_is_mod: false,
          self_is_enabled: false,
          self_is_effectively_active: false,
          self_owner_object_id: null,
          self_owner_object_folder_path: null,
          self_classification_reasons: [],
          conflicts: [],
          ancestor_disabled_by: null,
          ancestor_disabled_path: null,
          inactive_reason: null,
        },
        objects: [],
      },
      isLoading: false,
      isError: false,
      isPlaceholderData: false,
    })),
  }));

vi.mock('@/features/workspace-runtime/hooks/useWorkspaceViewModel', () => ({
  useWorkspaceViewModel: (options?: unknown) => mockUseWorkspaceViewModel(options),
}));

vi.mock('@/features/workspace-runtime/hooks/useWorkspaceExplorerPages', () => ({
  useWorkspaceExplorerPages: (query: unknown, options?: unknown) =>
    mockUseWorkspaceExplorerPages(query, options),
}));

vi.mock('../../../shared/lib/hooks/useFileDrop', () => ({
  useFileDrop: () => ({ isDragging: false, dragPosition: null }),
}));

vi.mock('../../../shared/lib/hooks/useDragAutoScroll', () => ({
  useDragAutoScroll: vi.fn(),
}));

vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: { id: 'test-game', mod_path: '/mods' } }),
}));

describe('useFolderGrid array bounds (TC-14)', () => {
  beforeEach(() => {
    mockUseWorkspaceViewModel.mockClear();
    mockUseWorkspaceExplorerPages.mockReset();
    mockListingRevision.current = 'revision-1';
    mockUseWorkspaceExplorerPages.mockImplementation(() => ({
      items: mockUseWorkspaceViewModel().data.explorer.children,
      totalMatching: 4,
      listingRevision: mockListingRevision.current,
      hasNextPage: false,
      isFetchingNextPage: false,
      isLoading: false,
      isError: false,
      error: null,
      fetchNextPage: vi.fn(),
    }));
    useAppStore.setState({
      gridSelection: new Set(),
      selectedModPath: null,
      explorerSearchQuery: '',
      sortField: 'name',
      sortOrder: 'asc',
      viewMode: 'grid',
    });
  });

  it('TC-14: handles Shift-Click bounds selection gracefully', () => {
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });

    // 1. Initial selection
    act(() => {
      result.current.toggleGridSelection('/Mod A', false, false);
    });

    expect(useAppStore.getState().gridSelection.has('/Mod A')).toBe(true);
    expect(useAppStore.getState().gridSelection.size).toBe(1);

    // 2. Shift-click to select a range (Mod A to Mod C)
    act(() => {
      // Simulate shift click on Mod C
      result.current.toggleGridSelection('/Mod C', true, true);
    });

    const currentSelection = Array.from(useAppStore.getState().gridSelection);
    expect(currentSelection).toContain('/Mod A');
    expect(currentSelection).toContain('/Mod B');
    expect(currentSelection).toContain('/Mod C');
    expect(currentSelection).not.toContain('/Mod D');
    expect(currentSelection.length).toBe(3);
  });

  it('uses the toggle action instance for folder switch pending state', () => {
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });

    expect(result.current.getFolderPendingDesiredEnabled).toBe(
      result.current.getPendingDesiredEnabled,
    );
  });

  it('keeps the current selection when activating another item is blocked by a dirty preview', () => {
    useAppStore.setState({
      gridSelection: new Set(['/Mod A']),
      selectedModPath: '/Mod A',
      workspacePreviewDirty: true,
      workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
      workspaceDialogState: { kind: 'none' },
    });
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });

    act(() => result.current.activateGridItem('/Mod B'));

    expect(useAppStore.getState().workspacePreviewTransition).toMatchObject({
      kind: 'pending',
      pendingTarget: {
        kind: 'selectMod',
        path: '/Mod B',
        selectionEffect: { gridSelection: [] },
      },
    });
    expect(useAppStore.getState().workspaceDialogState).toEqual({ kind: 'previewUnsavedChanges' });
    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod A']));
    expect(useAppStore.getState().selectedModPath).toBe('/Mod A');

    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({ type: 'PREVIEW_TRANSITION_CANCELLED' }),
    );

    expect(useAppStore.getState().workspacePreviewDirty).toBe(true);
    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod A']));
    expect(useAppStore.getState().selectedModPath).toBe('/Mod A');
  });

  it('does not replace grid selection before a dirty-preview toggle is confirmed', () => {
    useAppStore.setState({
      gridSelection: new Set(['/Mod A']),
      selectedModPath: '/Mod A',
      workspacePreviewDirty: true,
      workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
      workspaceDialogState: { kind: 'none' },
    });
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });

    act(() => result.current.toggleGridSelection('/Mod B', false));

    expect(useAppStore.getState().workspacePreviewTransition).toMatchObject({
      kind: 'pending',
      pendingTarget: {
        kind: 'selectMod',
        path: '/Mod B',
        selectionEffect: { gridSelection: ['/Mod B'] },
      },
    });
    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod A']));
    expect(useAppStore.getState().selectedModPath).toBe('/Mod A');

    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({ type: 'PREVIEW_TRANSITION_CANCELLED' }),
    );

    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod A']));
    expect(useAppStore.getState().selectedModPath).toBe('/Mod A');
  });

  it('commits activated item preview and cleared grid selection together after confirmation', () => {
    useAppStore.setState({
      gridSelection: new Set(['/Mod A']),
      selectedModPath: '/Mod A',
      workspacePreviewDirty: true,
      workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
      workspaceDialogState: { kind: 'none' },
    });
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });

    act(() => result.current.activateGridItem('/Mod B'));
    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({ type: 'PREVIEW_TRANSITION_CONFIRMED' }),
    );

    expect(useAppStore.getState().selectedModPath).toBe('/Mod B');
    expect(useAppStore.getState().gridSelection).toEqual(new Set());
  });

  it('commits checkbox preview and selection together after confirmation', () => {
    useAppStore.setState({
      gridSelection: new Set(['/Mod A']),
      selectedModPath: '/Mod A',
      workspacePreviewDirty: true,
      workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
      workspaceDialogState: { kind: 'none' },
    });
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });

    act(() => result.current.toggleGridSelection('/Mod B', false));
    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({ type: 'PREVIEW_TRANSITION_CONFIRMED' }),
    );

    expect(useAppStore.getState().selectedModPath).toBe('/Mod B');
    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod B']));
  });

  it('confirms an explicit pending selection across listing-only revisions', () => {
    mockListingRevision.current = null;
    useAppStore.setState({
      gridSelection: new Set(['/Mod A']),
      selectedModPath: '/Mod A',
      workspacePreviewDirty: true,
      workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
      workspaceDialogState: { kind: 'none' },
    });
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });

    act(() => result.current.toggleGridSelection('/Mod B', false));
    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod A']));

    mockListingRevision.current = 'revision-1';
    act(() => useAppStore.getState().setExplorerScrollOffset(10));
    mockListingRevision.current = 'revision-2';
    act(() => useAppStore.getState().setExplorerScrollOffset(20));

    expect(useAppStore.getState().workspacePreviewTransition.kind).toBe('pending');
    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({ type: 'PREVIEW_TRANSITION_CONFIRMED' }),
    );

    expect(useAppStore.getState().selectedModPath).toBe('/Mod B');
    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod B']));
  });

  it('keeps a dirty-preview confirmation through an unchanged selection reconciliation', () => {
    useAppStore.setState({
      selectedObjectFolderPath: 'ALBEDO',
      explorerSubPath: 'ALBEDO',
      currentPath: ['ALBEDO'],
      gridSelection: new Set(['/Mod A']),
      selectedModPath: '/Mod A',
      workspacePreviewDirty: true,
      workspacePreviewTransition: {
        kind: 'pending',
        pendingTarget: {
          kind: 'selectMod',
          path: '/Mod B',
          selectionEffect: { gridSelection: ['/Mod B'] },
        },
      },
      workspaceDialogState: { kind: 'previewUnsavedChanges' },
    });

    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({
        type: 'SELECTION_RECONCILED',
        selectedObjectFolderPath: 'ALBEDO',
        explorerSubPath: 'ALBEDO',
        selectedModPath: '/Mod A',
        currentPath: ['ALBEDO'],
        reconciliationStatus: 'unchanged',
        reconciliationReason: null,
        affectedPaths: [],
      }),
    );

    expect(useAppStore.getState().workspacePreviewTransition.kind).toBe('pending');
    expect(useAppStore.getState().workspaceDialogState).toEqual({ kind: 'previewUnsavedChanges' });
    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({ type: 'PREVIEW_TRANSITION_CONFIRMED' }),
    );

    expect(useAppStore.getState().selectedModPath).toBe('/Mod B');
    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod B']));
  });

  it('ignores a selection callback captured before the explorer scope changed', () => {
    useAppStore.setState({
      gridSelection: new Set(['/Mod A']),
      selectedModPath: '/Mod A',
      workspacePreviewDirty: true,
      workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
      workspaceDialogState: { kind: 'none' },
    });
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });

    act(() => result.current.activateGridItem('/Mod B'));
    const queuedTransition = useAppStore.getState().workspacePreviewTransition;
    if (
      queuedTransition.kind !== 'pending' ||
      queuedTransition.pendingTarget.kind !== 'selectMod' ||
      !queuedTransition.pendingTarget.selectionEffect
    ) {
      throw new Error('Expected activation to queue a selection effect');
    }
    const staleSelectionEffect = queuedTransition.pendingTarget.selectionEffect;

    act(() => result.current.setExplorerSearch('amber'));
    act(() => {
      useAppStore.setState({
        gridSelection: new Set(['/Mod A']),
        selectedModPath: '/Mod A',
        workspacePreviewDirty: false,
        workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
        workspaceDialogState: { kind: 'none' },
      });
    });
    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({
        type: 'MOD_SELECTED',
        path: '/Mod C',
        selectionEffect: staleSelectionEffect,
      }),
    );

    expect(useAppStore.getState().workspacePreviewTransition).toEqual({
      kind: 'idle',
      pendingTarget: null,
    });
    expect(useAppStore.getState().selectedModPath).toBe('/Mod A');
    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod A']));

    act(() => {
      useAppStore.setState({
        workspacePreviewDirty: true,
        workspacePreviewTransition: {
          kind: 'pending',
          pendingTarget: { kind: 'focusObject', folderPath: '/Other' },
        },
        workspaceDialogState: { kind: 'previewUnsavedChanges' },
      });
    });
    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({
        type: 'MOD_SELECTED',
        path: '/Mod C',
        selectionEffect: staleSelectionEffect,
      }),
    );

    expect(useAppStore.getState().selectedModPath).toBe('/Mod A');
    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod A']));
    expect(useAppStore.getState().workspacePreviewTransition).toEqual({
      kind: 'pending',
      pendingTarget: { kind: 'focusObject', folderPath: '/Other' },
    });
  });

  it('replaces a pending dirty-preview selection with the latest requested target', () => {
    useAppStore.setState({
      gridSelection: new Set(['/Mod A']),
      selectedModPath: '/Mod A',
      workspacePreviewDirty: true,
      workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
      workspaceDialogState: { kind: 'none' },
    });
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });

    act(() => result.current.toggleGridSelection('/Mod B', false));
    act(() => result.current.toggleGridSelection('/Mod C', false));

    expect(useAppStore.getState().selectedModPath).toBe('/Mod A');
    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod A']));
    expect(useAppStore.getState().workspacePreviewTransition).toMatchObject({
      kind: 'pending',
      pendingTarget: {
        kind: 'selectMod',
        path: '/Mod C',
        selectionEffect: { gridSelection: ['/Mod C'] },
      },
    });

    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({ type: 'PREVIEW_TRANSITION_CONFIRMED' }),
    );

    expect(useAppStore.getState().selectedModPath).toBe('/Mod C');
    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod C']));
  });

  it('keeps the original preview and grid selection when the latest pending target is cancelled', () => {
    useAppStore.setState({
      gridSelection: new Set(['/Mod A']),
      selectedModPath: '/Mod A',
      workspacePreviewDirty: true,
      workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
      workspaceDialogState: { kind: 'none' },
    });
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });

    act(() => result.current.toggleGridSelection('/Mod B', false));
    act(() => result.current.toggleGridSelection('/Mod C', false));
    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({ type: 'PREVIEW_TRANSITION_CANCELLED' }),
    );

    expect(useAppStore.getState().selectedModPath).toBe('/Mod A');
    expect(useAppStore.getState().gridSelection).toEqual(new Set(['/Mod A']));
    expect(useAppStore.getState().workspacePreviewTransition).toEqual({
      kind: 'idle',
      pendingTarget: null,
    });
  });

  it('clears local all-matching selection after a confirmed activation', async () => {
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });
    act(() => result.current.selectAllMatching());
    act(() => {
      useAppStore.setState({
        selectedModPath: '/Preview',
        workspacePreviewDirty: true,
        workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
        workspaceDialogState: { kind: 'none' },
      });
    });

    act(() => result.current.activateGridItem('/Mod B'));
    expect(result.current.explorerSelection.mode).toBe('all_matching');
    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({ type: 'PREVIEW_TRANSITION_CONFIRMED' }),
    );

    await waitFor(() => {
      expect(result.current.explorerSelection).toEqual({ mode: 'explicit', paths: new Set() });
      expect(useAppStore.getState().gridSelection).toEqual(new Set());
      expect(useAppStore.getState().selectedModPath).toBe('/Mod B');
    });
  });

  it('commits an all-matching checkbox exclusion only after confirmation', async () => {
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });
    act(() => result.current.selectAllMatching());
    act(() => {
      useAppStore.setState({
        selectedModPath: '/Preview',
        workspacePreviewDirty: true,
        workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
        workspaceDialogState: { kind: 'none' },
      });
    });

    act(() => result.current.toggleGridSelection('/Mod B', true));
    expect(result.current.isPathSelected('/Mod B')).toBe(true);
    expect(useAppStore.getState().gridSelection).toEqual(new Set());

    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({ type: 'PREVIEW_TRANSITION_CONFIRMED' }),
    );

    await waitFor(() => expect(result.current.isPathSelected('/Mod B')).toBe(false));
    expect(useAppStore.getState().gridSelection).toEqual(new Set());
    expect(useAppStore.getState().selectedModPath).toBe('/Mod A');
  });

  it('keeps all-matching exclusions unchanged when a guarded checkbox transition is cancelled', async () => {
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });
    act(() => result.current.selectAllMatching());
    act(() => {
      useAppStore.setState({
        selectedModPath: '/Preview',
        workspacePreviewDirty: true,
        workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
        workspaceDialogState: { kind: 'none' },
      });
    });

    act(() => result.current.toggleGridSelection('/Mod B', true));
    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({ type: 'PREVIEW_TRANSITION_CANCELLED' }),
    );

    await waitFor(() => expect(result.current.isPathSelected('/Mod B')).toBe(true));
    expect(useAppStore.getState().selectedModPath).toBe('/Preview');
    expect(useAppStore.getState().gridSelection).toEqual(new Set());
  });

  it('does not restore an all-matching candidate after its search scope changes', async () => {
    const { result } = renderHook(() => useFolderGrid(), { wrapper: createWrapper });
    act(() => result.current.selectAllMatching());
    act(() => {
      useAppStore.setState({
        selectedModPath: '/Preview',
        workspacePreviewDirty: true,
        workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
        workspaceDialogState: { kind: 'none' },
      });
    });

    act(() => result.current.toggleGridSelection('/Mod B', true));
    expect(useAppStore.getState().workspacePreviewTransition.kind).toBe('pending');
    act(() => result.current.setExplorerSearch('amber'));
    await waitFor(() => expect(result.current.explorerSelection.mode).toBe('explicit'));
    const pendingTransition = useAppStore.getState().workspacePreviewTransition;
    expect(pendingTransition).toMatchObject({
      kind: 'pending',
      pendingTarget: {
        kind: 'selectMod',
        selectionEffect: {
          gridSelection: [],
          isCurrent: expect.any(Function),
          onApplied: expect.any(Function),
        },
      },
    });
    if (
      pendingTransition.kind === 'pending' &&
      pendingTransition.pendingTarget.kind === 'selectMod'
    ) {
      expect(pendingTransition.pendingTarget.selectionEffect?.isCurrent?.()).toBe(false);
    }
    act(() =>
      useAppStore.getState().dispatchWorkspaceRuntime({ type: 'PREVIEW_TRANSITION_CONFIRMED' }),
    );

    await waitFor(() => {
      expect(result.current.explorerSelection).toEqual({ mode: 'explicit', paths: new Set() });
      expect(useAppStore.getState().gridSelection).toEqual(new Set());
      expect(useAppStore.getState().workspacePreviewTransition.kind).toBe('idle');
    });
  });

  it('loads the previous breadcrumb folder with a separate paged query', () => {
    useAppStore.setState({
      currentPath: ['SkinSelectImpact', 'Aglaea'],
      explorerSubPath: 'SkinSelectImpact/Aglaea',
    });

    renderHook(() => useFolderGrid(), { wrapper: createWrapper });

    expect(mockUseWorkspaceExplorerPages).toHaveBeenCalledWith(
      expect.objectContaining({
        explorer_sub_path: 'SkinSelectImpact',
        search_query: null,
        sort_field: 'name',
        sort_order: 'asc',
        safety_filter: 'all',
      }),
      { enabled: true },
    );
  });
});
