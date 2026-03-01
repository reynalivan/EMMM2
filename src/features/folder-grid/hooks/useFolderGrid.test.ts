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

vi.mock('../../../hooks/useObjects', () => ({
  useObjects: () => ({ data: [] }),
}));

vi.mock('../../../hooks/useFolders', () => ({
  useModFolders: () => ({
    data: {
      children: [
        { name: 'Mod A', path: '/Mod A', folder_name: 'Mod A' },
        { name: 'Mod B', path: '/Mod B', folder_name: 'Mod B' },
        { name: 'Mod C', path: '/Mod C', folder_name: 'Mod C' },
        { name: 'Mod D', path: '/Mod D', folder_name: 'Mod D' },
      ] as ModFolder[],
    },
    isLoading: false,
  }),
  sortFolders: (f: ModFolder[]) => f,
  folderKeys: { all: [] },
  useImportMods: () => ({ mutate: vi.fn() }),
  useToggleMod: () => ({ mutate: vi.fn() }),
  useRenameMod: () => ({ mutate: vi.fn() }),
  useDeleteMod: () => ({ mutate: vi.fn() }),
  useEnableOnlyThis: () => ({ mutate: vi.fn() }),
  useUpdateInfo: () => ({ mutate: vi.fn() }),
  useCheckDuplicate: () => ({ mutate: vi.fn() }),
  useToggleModSafe: () => ({ mutate: vi.fn() }),
  useBulkToggle: () => ({ mutate: vi.fn() }),
  useBulkDelete: () => ({ mutate: vi.fn() }),
  useBulkUpdateInfo: () => ({ mutate: vi.fn() }),
}));

vi.mock('../../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({ activeGame: { id: 'test-game', mod_path: '/mods' } }),
}));

describe('useFolderGrid array bounds (TC-14)', () => {
  beforeEach(() => {
    useAppStore.setState({
      gridSelection: new Set(),
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
