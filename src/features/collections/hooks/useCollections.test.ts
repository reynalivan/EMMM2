import React from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createWrapper } from '../../../testing/test-utils';
import { corridorKeys } from '../queryKeys';
import { useApplyCollection, useCollections } from './useCollections';
import type { CorridorSnapshot } from '../../../types/collection';

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
  });

  it('loads collections for active game id', async () => {
    vi.mocked(invoke).mockResolvedValue([
      {
        id: 'c-1',
        name: 'Abyss Team',
        game_id: 'g-1',
        is_safe: true,
        member_count: 5,
      },
    ]);

    const { result } = renderHook(() => useCollections('g-1', true), {
      wrapper: createWrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(invoke).toHaveBeenCalledWith('list_collections', { gameId: 'g-1' });
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
          changed_count: 1,
          warnings: [],
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
          undo_collection_id: null,
          current_signature: 'backend-sig',
          is_dirty: false,
          last_switched_at: null,
        } satisfies CorridorSnapshot;
      }

      if (command === 'get_collection_runtime_preview') {
        return {
          collection: {
            id: 'c-1',
            name: 'Collection One',
            game_id: 'g-1',
            is_safe: true,
            member_count: 1,
            is_last_unsaved: false,
          },
          roots: [],
          object_states: [],
          signature: 'preview-sig',
        };
      }

      throw new Error(`Unexpected invoke command: ${command}`);
    });

    queryClient.setQueryData<CorridorSnapshot>(corridorKeys.state('g-1', true), {
      game_id: 'g-1',
      is_safe: true,
      active_collection_id: null,
      active_collection_name: 'Old Snapshot',
      undo_collection_id: null,
      current_signature: 'old',
      is_dirty: false,
      last_switched_at: null,
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
});
