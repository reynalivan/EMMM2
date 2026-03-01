import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useObjectListHandlers } from './useObjectListHandlers';
import { useToggleMod, useDeleteMod } from '../../hooks/useFolders';
import { useDeleteObject, useUpdateObject } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import { invoke } from '@tauri-apps/api/core';
import { scanService } from '../../lib/services/scanService';
// toast store mock unused
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';

// Mocks
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../hooks/useFolders', () => ({
  useToggleMod: vi.fn(),
  useDeleteMod: vi.fn(),
}));

vi.mock('../../hooks/useObjects', () => ({
  useDeleteObject: vi.fn(),
  useUpdateObject: vi.fn(),
}));

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: vi.fn(),
}));

vi.mock('../../lib/services/scanService', () => ({
  scanService: {
    scanPreview: vi.fn(),
    getMasterDb: vi.fn(),
    commitScan: vi.fn(),
    extractArchive: vi.fn(),
  },
}));

vi.mock('../../stores/useToastStore', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

vi.mock('../../stores/useAppStore', () => ({
  useAppStore: {
    getState: vi.fn(() => ({
      explorerSubPath: '',
      setExplorerSubPath: vi.fn(),
      setCurrentPath: vi.fn(),
    })),
  },
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
};

describe('useObjectListHandlers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const defaultProps = {
    objects: [
      {
        id: 'obj-1',
        name: 'Object 1',
        object_type: 'Character',
        is_pinned: false,
        thumbnail_path: null,
        folder_path: '',
        sub_category: null,
        mod_count: 0,
        enabled_count: 0,
        is_safe: true,
        is_auto_sync: false,
        tags: '',
        metadata: '{}',
        has_naming_conflict: false,
      },
    ],
    folders: [],
    schema: {
      categories: [{ name: 'Character', label: 'Characters' }],
    } as unknown as import('../../types/object').GameSchema,
  };

  it('handleToggle calls toggleMod mutation', () => {
    const mockToggleMutate = vi.fn();
    vi.mocked(useToggleMod).mockReturnValue({ mutate: mockToggleMutate } as unknown as ReturnType<
      typeof useToggleMod
    >);
    vi.mocked(useDeleteMod).mockReturnValue({} as unknown as ReturnType<typeof useDeleteMod>);
    vi.mocked(useDeleteObject).mockReturnValue({} as unknown as ReturnType<typeof useDeleteObject>);
    vi.mocked(useUpdateObject).mockReturnValue({} as unknown as ReturnType<typeof useUpdateObject>);
    vi.mocked(useActiveGame).mockReturnValue({
      activeGame: { id: 'game-1', mod_path: 'C:\\mods' },
    } as unknown as ReturnType<typeof useActiveGame>);

    const { result } = renderHook(() => useObjectListHandlers(defaultProps), {
      wrapper: createWrapper(),
    });

    act(() => {
      result.current.handleToggle('C:\\mods\\folder1', false);
    });

    expect(mockToggleMutate).toHaveBeenCalledWith(
      { path: 'C:\\mods\\folder1', enable: true, gameId: 'game-1' },
      expect.any(Object),
    );
  });

  it('handleOpen invokes open_in_explorer', async () => {
    vi.mocked(useToggleMod).mockReturnValue({} as unknown as ReturnType<typeof useToggleMod>);
    vi.mocked(useDeleteMod).mockReturnValue({} as unknown as ReturnType<typeof useDeleteMod>);
    vi.mocked(useDeleteObject).mockReturnValue({} as unknown as ReturnType<typeof useDeleteObject>);
    vi.mocked(useUpdateObject).mockReturnValue({} as unknown as ReturnType<typeof useUpdateObject>);
    vi.mocked(useActiveGame).mockReturnValue({ activeGame: null } as unknown as ReturnType<
      typeof useActiveGame
    >);

    const { result } = renderHook(() => useObjectListHandlers(defaultProps), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.handleOpen('C:\\mods');
    });

    expect(invoke).toHaveBeenCalledWith('open_in_explorer', { path: 'C:\\mods' });
  });

  it('handleDelete directly mutates if folder is empty', async () => {
    const mockDeleteMutate = vi.fn();
    vi.mocked(useToggleMod).mockReturnValue({} as unknown as ReturnType<typeof useToggleMod>);
    vi.mocked(useDeleteMod).mockReturnValue({ mutate: mockDeleteMutate } as unknown as ReturnType<
      typeof useDeleteMod
    >);
    vi.mocked(useDeleteObject).mockReturnValue({} as unknown as ReturnType<typeof useDeleteObject>);
    vi.mocked(useUpdateObject).mockReturnValue({} as unknown as ReturnType<typeof useUpdateObject>);
    vi.mocked(useActiveGame).mockReturnValue({
      activeGame: { id: 'game-1' },
    } as unknown as ReturnType<typeof useActiveGame>);

    // Mock pre_delete_check returning empty
    vi.mocked(invoke).mockResolvedValue({
      is_empty: true,
      item_count: 0,
      name: 'folder1',
      path: 'C:\\mods\\folder1',
    });

    const { result } = renderHook(() => useObjectListHandlers(defaultProps), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.handleDelete('C:\\mods\\folder1');
    });

    expect(invoke).toHaveBeenCalledWith('pre_delete_check', { path: 'C:\\mods\\folder1' });
    expect(mockDeleteMutate).toHaveBeenCalledWith({ path: 'C:\\mods\\folder1', gameId: 'game-1' });
  });

  it('handleSync triggers scan preview flow', async () => {
    vi.mocked(useToggleMod).mockReturnValue({} as unknown as ReturnType<typeof useToggleMod>);
    vi.mocked(useDeleteMod).mockReturnValue({} as unknown as ReturnType<typeof useDeleteMod>);
    vi.mocked(useDeleteObject).mockReturnValue({} as unknown as ReturnType<typeof useDeleteObject>);
    vi.mocked(useUpdateObject).mockReturnValue({} as unknown as ReturnType<typeof useUpdateObject>);
    vi.mocked(useActiveGame).mockReturnValue({
      activeGame: { id: 'game-1', game_type: 'hsr', mod_path: 'C:\\mods' },
    } as unknown as ReturnType<typeof useActiveGame>);

    vi.mocked(scanService.scanPreview).mockResolvedValue([
      { folderName: 'mod1' } as unknown as Awaited<ReturnType<typeof scanService.scanPreview>>[0],
    ]);
    vi.mocked(scanService.getMasterDb).mockResolvedValue('[]');

    const { result } = renderHook(() => useObjectListHandlers(defaultProps), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.handleSync();
    });

    expect(scanService.scanPreview).toHaveBeenCalledWith('game-1', 'hsr', 'C:\\mods');
    expect(scanService.getMasterDb).toHaveBeenCalledWith('hsr');

    expect(result.current.scanReview.open).toBe(true);
    expect(result.current.scanReview.items).toHaveLength(1);
  });
});
