import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useObjectListHandlers } from './useObjectListHandlers';
import { useDeleteMod } from '../../hooks/useFolderCoreMutations';
import { useDeleteObject, useUpdateObject } from '../../hooks/useObjectMutations';
import { useActiveGame } from '../../hooks/useActiveGame';
import { scanService } from '../../lib/services/scanService';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';

// Mocks
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../hooks/useFolderCoreMutations', () => ({
  useDeleteMod: vi.fn(),
}));

vi.mock('../../hooks/useObjectMutations', () => ({
  useDeleteObject: vi.fn(),
  useUpdateObject: vi.fn(),
}));

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: vi.fn(),
}));

vi.mock('../../lib/services/scanService', () => ({
  scanService: {
    runDeepmatchPreview: vi.fn(),
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
        display_name: 'Object 1',
        node_kind: 'object' as const,
        display_mode: 'unknown' as const,
        type_chip: null,
        object_type: 'Character',
        is_pinned: false,
        thumbnail_path: null,
        folder_path: '',
        sub_category: null,
        mod_count: 0,
        enabled_count: 0,
        tags: '[]',
        metadata: '{}',
        is_auto_sync: false,
        is_object_disabled: false,
        status: 1,
        created_at: '2025-01-01T00:00:00Z',
        hash_db: null,
        custom_skins: null,
        has_naming_conflict: false,
        inactive_reason: null,
        is_effectively_active: false,
        warning_state: 'none' as const,
        primary_warning: null,
        switch_state: 'disabled' as const,
        switch_reason: null,
        switch_policy_key: 'object' as const,
        capabilities: {
          can_toggle: false,
          can_rename: true,
          can_delete: true,
          can_move: false,
          can_toggle_safe: false,
          can_sync: true,
          can_enable_only_this: false,
          can_pin: true,
          can_edit_metadata: true,
          can_reveal_in_explorer: true,
          can_move_category: true,
          can_open_in_explorer: true,
        },
      },
    ],
    schema: {
      categories: [{ name: 'Character', label: 'Characters' }],
    } as unknown as import('../../types/object').GameSchema,
    mismatchConfirm: null,
    setMismatchConfirm: vi.fn(),
  };

  it('handleSync triggers scan preview flow', async () => {
    vi.mocked(useDeleteMod).mockReturnValue({} as unknown as ReturnType<typeof useDeleteMod>);
    vi.mocked(useDeleteObject).mockReturnValue({} as unknown as ReturnType<typeof useDeleteObject>);
    vi.mocked(useUpdateObject).mockReturnValue({} as unknown as ReturnType<typeof useUpdateObject>);
    vi.mocked(useActiveGame).mockReturnValue({
      activeGame: { id: 'game-1', game_type: 'hsr', mod_path: 'C:\\mods' },
    } as unknown as ReturnType<typeof useActiveGame>);

    vi.mocked(scanService.runDeepmatchPreview).mockResolvedValue([
      { folderName: 'mod1' } as unknown as Awaited<
        ReturnType<typeof scanService.runDeepmatchPreview>
      >[0],
    ]);
    vi.mocked(scanService.getMasterDb).mockResolvedValue('[]');

    const { result } = renderHook(() => useObjectListHandlers(defaultProps), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.handleSync();
    });

    expect(scanService.runDeepmatchPreview).toHaveBeenCalledWith('game-1', 'hsr', 'C:\\mods');
    expect(scanService.getMasterDb).toHaveBeenCalledWith('hsr');

    expect(result.current.scanReview.open).toBe(true);
    expect(result.current.scanReview.items).toHaveLength(1);
  });
});
