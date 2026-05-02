import React from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createWrapper } from '../../../testing/test-utils';
import { corridorKeys } from '../queryKeys';
import { useApplyCollection, useApplyCollectionPreview, useCollections } from './useCollections';
import type { CorridorSnapshot } from '../../../types/collection';
import { useAppStore } from '../../../stores/useAppStore';

function createProjectedState() {
  return {
    object_states: [],
    active_roots: [],
    summary: {
      object_count: 0,
      enabled_object_count: 0,
      active_root_count: 0,
      missing_root_count: 0,
    },
  };
}

// Restore real @tanstack/react-query — the global setupTests stub
// replaces useQuery with a no-op, which means queryFn never runs.
vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../../stores/useToastStore', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    withAction: vi.fn(),
  },
}));

function createMutationWrapper(queryClient: QueryClient) {
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
}

describe('useCollections', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      workspaceDialogState: { kind: 'none' },
    });
  });

  it('loads collections for active game id', async () => {
    vi.mocked(invoke).mockResolvedValue([
      {
        id: 'c-1',
        name: 'Abyss Team',
        game_id: 'g-1',
        is_safe: true,
        member_count: 5,
        mod_count: 5,
      },
    ]);

    const { result } = renderHook(() => useCollections('g-1', true), {
      wrapper: createWrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(invoke).toHaveBeenCalledWith('list_collections', { gameId: 'g-1', isSafe: true });
    expect(result.current.data?.[0].name).toBe('Abyss Team');
  });

  it('apply resolves with backend-authoritative runtime snapshot', async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });
    const wrapper = createMutationWrapper(queryClient);

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'apply_collection') {
        return {
          success: true,
          mods_enabled: 1,
          mods_disabled: 0,
          objects_toggled: 0,
          undo_collection_id: null,
          new_signature: 'backend-sig',
          warnings: [],
          final_state_name: 'Backend Runtime',
          final_mode: 'SAFE',
        };
      }

      if (command === 'list_collections') {
        return [];
      }

      if (command === 'get_corridor_state') {
        return {
          game_id: 'g-1',
          is_safe: true,
          active_collection_id: 'c-1',
          active_collection_name: 'Backend Runtime',
          active_collection_is_unsaved: false,
          undo_collection_id: null,
          current_signature: 'backend-sig',
          is_dirty: false,
          last_switched_at: null,
          current_mods: [],
          current_objects: [],
          current_tree_nodes: [],
          projected_state: createProjectedState(),
        } satisfies CorridorSnapshot;
      }

      throw new Error(`Unexpected invoke command: ${command}`);
    });

    queryClient.setQueryData<CorridorSnapshot>(corridorKeys.state('g-1', true), {
      game_id: 'g-1',
      is_safe: true,
      active_collection_id: null,
      active_collection_name: 'Old Snapshot',
      active_collection_is_unsaved: false,
      undo_collection_id: null,
      current_signature: 'old',
      is_dirty: false,
      last_switched_at: null,
      current_mods: [],
      current_objects: [],
      current_tree_nodes: [],
      projected_state: createProjectedState(),
    });

    const { result } = renderHook(() => useApplyCollection(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({
        collectionId: 'c-1',
        gameId: 'g-1',
      });
    });

    const snapshot = queryClient.getQueryData<CorridorSnapshot>(corridorKeys.state('g-1', true));
    expect(snapshot?.active_collection_name).toBe('Backend Runtime');
  });

  it('refetches apply preview when game id changes', async () => {
    vi.mocked(invoke).mockResolvedValue({
      collection_name: 'Preset',
      current_snapshot: null,
      current_mods: [],
      current_objects: [],
      current_tree_nodes: [],
      target_mods: [],
      target_objects: [],
      target_tree_nodes: [],
      current_state_name: null,
      current_state_is_unsaved: false,
      current_projected_state: createProjectedState(),
      target_projected_state: createProjectedState(),
    });

    const { rerender } = renderHook(
      ({ gameId }: { gameId: string }) => useApplyCollectionPreview(gameId, 'c-1', true),
      {
        initialProps: { gameId: 'g-1' },
        wrapper: createWrapper,
      },
    );

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('preview_apply_collection', {
        gameId: 'g-1',
        collectionId: 'c-1',
        isSafe: true,
      }),
    );

    vi.mocked(invoke).mockClear();
    rerender({ gameId: 'g-2' });

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('preview_apply_collection', {
        gameId: 'g-2',
        collectionId: 'c-1',
        isSafe: true,
      }),
    );
  });

  it('opens file-in-use through workspace runtime dialog on apply error', async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });
    const wrapper = createMutationWrapper(queryClient);

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'apply_collection') {
        throw new Error(
          JSON.stringify({
            type: 'FileInUse',
            payload: {
              path: 'Mods/Alpha',
              processes: ['explorer.exe'],
            },
          }),
        );
      }

      throw new Error(`Unexpected invoke command: ${command}`);
    });

    const { result } = renderHook(() => useApplyCollection(), { wrapper });

    await act(async () => {
      try {
        await result.current.mutateAsync({
          collectionId: 'c-1',
          gameId: 'g-1',
        });
      } catch {
        // Mutation error is handled by onError; dialog state is the assertion target.
      }
    });

    expect(useAppStore.getState().workspaceDialogState).toEqual({
      kind: 'fileInUse',
      data: {
        path: 'Mods/Alpha',
        processes: ['explorer.exe'],
        onRetry: expect.any(Function),
      },
    });
  });
});
