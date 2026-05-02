import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useFolderGrid } from './useFolderGrid';
import { useAppStore } from '../../../stores/useAppStore';
import { createWrapper } from '../../../testing/test-utils';
import { ModFolder } from '../../../types/mod';

// Provide element dimensions for virtualization
globalThis.ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
};

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));

vi.mock('../../../hooks/folderCache', () => ({
  sortFolders: (f: ModFolder[]) => f,
  folderKeys: { all: [] },
}));

vi.mock('../../../hooks/useFolderCoreMutations', () => ({
  useRenameMod: () => ({ mutate: vi.fn() }),
  useDeleteMod: () => ({ mutate: vi.fn() }),
}));

vi.mock('../../../hooks/useFolderMutations', () => ({
  useImportMods: () => ({ mutate: vi.fn() }),
  useToggleModSafe: () => ({ mutate: vi.fn() }),
  useBulkToggle: () => ({ mutate: vi.fn() }),
  useBulkDelete: () => ({ mutate: vi.fn() }),
  useBulkUpdateInfo: () => ({ mutate: vi.fn() }),
  useBulkFavorite: () => ({ mutate: vi.fn() }),
  useBulkPin: () => ({ mutate: vi.fn() }),
  useUpdateModInfo: () => ({ mutate: vi.fn() }),
  useActiveConflicts: () => ({ data: [] }),
}));

vi.mock('../../workspace-runtime/useWorkspaceViewModel', () => ({
  useWorkspaceViewModel: () => ({
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
  }),
}));

vi.mock('../../../hooks/useFileDrop', () => ({
  useFileDrop: () => ({ isDragging: false, dragPosition: null }),
}));

vi.mock('../../../hooks/useDragAutoScroll', () => ({
  useDragAutoScroll: vi.fn(),
}));

vi.mock('../../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({ activeGame: { id: 'test-game', mod_path: '/mods' } }),
}));

describe('useFolderGrid array bounds (TC-14)', () => {
  beforeEach(() => {
    useAppStore.setState({
      gridSelection: new Set(),
      selectedModPath: null,
      safeMode: false,
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
});
