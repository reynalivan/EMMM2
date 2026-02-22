import { renderHook, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useModFolders } from './useFolders';
import { createWrapper } from '../test-utils';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock useAppStore
vi.mock('../stores/useAppStore', () => ({
  useAppStore: vi.fn(() => ({
    sortField: 'name',
    sortOrder: 'asc',
    explorerSearchQuery: '',
    viewMode: 'grid',
  })),
}));

// Mock useActiveGame
vi.mock('./useActiveGame', () => ({
  useActiveGame: vi.fn(() => ({
    activeGame: {
      id: 'game1',
      mod_path: '/mods',
    },
  })),
}));

describe('useModFolders', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('fetches mod folders successfully', async () => {
    const mockData = [
      {
        path: '/mods/ModA',
        name: 'ModA',
        folder_name: 'ModA',
        is_enabled: true,
        is_directory: true,
        modified_at: 100,
        size_bytes: 1024,
      },
    ];

    vi.mocked(invoke).mockResolvedValue(mockData);

    // Call without args to test root listing (subPath: null)
    const { result } = renderHook(() => useModFolders(), {
      wrapper: createWrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockData);
    expect(invoke).toHaveBeenCalledWith('list_mod_folders', {
      gameId: 'game1',
      modsPath: '/mods',
      subPath: null,
      objectId: null,
    });
  });

  it.skip('handles errors', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('Failed to list'));

    const { result } = renderHook(() => useModFolders(), {
      wrapper: createWrapper,
    });

    await waitFor(() => expect(result.current.isError).toBe(true));
    expect(result.current.error).toBeDefined();
  });
});
