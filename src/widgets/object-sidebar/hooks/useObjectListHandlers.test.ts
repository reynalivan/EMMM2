import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useObjectListHandlers } from './useObjectListHandlers';
import { useDeleteMod } from '@/features/mod-runtime';
import { useDeleteObject, useUpdateObject } from '@/features/workspace-runtime';
import { useActiveGame } from '@/entities/game';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';

// Mocks
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@/features/mod-runtime', () => ({
  useDeleteMod: vi.fn(),
}));

vi.mock('@/features/workspace-runtime', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/features/workspace-runtime')>()),
  useDeleteObject: vi.fn(),
  useUpdateObject: vi.fn(),
}));

vi.mock('@/entities/game', () => ({
  useActiveGame: vi.fn(),
}));

const openObjectClassificationWizard = vi.fn();
vi.mock('@/features/import-batches/classificationLauncher', () => ({
  openObjectClassificationWizard: (...args: unknown[]) => openObjectClassificationWizard(...args),
}));

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

vi.mock('@/app/store', () => ({
  useAppStore: Object.assign(
    vi.fn(
      (
        selector?: (state: {
          selectedObjectFolderPath: string | null;
          explorerSubPath: string | undefined;
          currentPath: string[];
          selectedModPath: string | null;
          mobileActivePane: 'sidebar' | 'grid' | 'details';
          workspacePreviewDirty: boolean;
          workspacePreviewTransition: { kind: 'idle'; pendingTarget: null };
          workspaceDialogState: { kind: 'none' };
          setExplorerSubPath: ReturnType<typeof vi.fn>;
          setCurrentPath: ReturnType<typeof vi.fn>;
        }) => unknown,
      ) => {
        const state = {
          selectedObjectFolderPath: null,
          explorerSubPath: '',
          currentPath: [],
          selectedModPath: null,
          mobileActivePane: 'sidebar' as const,
          workspacePreviewDirty: false,
          workspacePreviewTransition: { kind: 'idle' as const, pendingTarget: null },
          workspaceDialogState: { kind: 'none' as const },
          setExplorerSubPath: vi.fn(),
          setCurrentPath: vi.fn(),
        };

        if (!selector) {
          return state;
        }

        return selector(state);
      },
    ),
    {
      getState: vi.fn(() => ({
        selectedObjectFolderPath: null,
        explorerSubPath: '',
        currentPath: [],
        selectedModPath: null,
        mobileActivePane: 'sidebar',
        workspacePreviewDirty: false,
        workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
        workspaceDialogState: { kind: 'none' },
        setExplorerSubPath: vi.fn(),
        setCurrentPath: vi.fn(),
      })),
      setState: vi.fn(),
    },
  ),
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
        matched_entry_key: null,
        matched_alias_name: null,
        matched_confidence: null,
        matched_reason: null,
        matched_source: null,
        active_mod_paths: null,
        name: 'Object 1',
        display_name: 'Object 1',
        is_registered: true,
        node_kind: 'object' as const,
        display_mode: 'unknown' as const,
        type_chip: null,
        object_type: 'Character',
        randomizer_mode: null,
        is_pinned: false,
        thumbnail_path: null,
        folder_path: '',
        sub_category: null,
        mod_count: 0,
        enabled_count: 0,
        safe_mod_count: 0,
        unsafe_mod_count: 0,
        unclassified_mod_count: 0,
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
    } as unknown as import('@/entities/game-object/model/object').GameSchema,
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

    const { result } = renderHook(() => useObjectListHandlers(defaultProps), {
      wrapper: createWrapper(),
    });

    await act(async () => {
      await result.current.handleSync();
    });

    expect(openObjectClassificationWizard).toHaveBeenCalledWith({
      gameId: 'game-1',
      objectIds: ['obj-1'],
    });
  });
});
