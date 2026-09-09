import React from 'react';
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { describe, expect, it, vi } from 'vitest';
import type { CollectionRuntimeSnapshot } from '@/entities/collection';
import { useCollectionRuntime } from './useCollectionRuntime';

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

function createWrapper(queryClient: QueryClient) {
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
}

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });

  return { promise, resolve };
}

function createRuntimeSnapshot(gameId: string): CollectionRuntimeSnapshot {
  return {
    game_id: gameId,
    active_collection_id: `collection-${gameId}`,
    active_collection_name: `Collection ${gameId}`,
    current_signature: `signature-${gameId}`,
    is_dirty: false,
    runtime_status: 'clean',
    is_safe: true,
    is_safety_classified: true,
    missing_count: 0,
    last_changes: null,
    current_mods: [],
    current_objects: [
      {
        kind: 'object',
        collection_id: `collection-${gameId}`,
        object_id: `object-${gameId}`,
        is_enabled: true,
        display_name: `Object ${gameId}`,
        path_key: `mods/${gameId}`,
      },
    ],
    current_tree_nodes: [],
    projected_state: {
      object_states: [],
      active_roots: [],
      summary: {
        object_count: 0,
        enabled_object_count: 0,
        active_root_count: 0,
        missing_root_count: 0,
      },
    },
  };
}

describe('useCollectionRuntime', () => {
  it('does not retain game A object state while game B is still loading', async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const gameBRuntime = createDeferred<unknown>();
    vi.mocked(invoke).mockImplementation((command, args) => {
      if (command !== 'get_collection_runtime_state') {
        throw new Error(`Unexpected invoke command: ${command}`);
      }

      const { gameId } = args as { gameId: string };
      return gameId === 'game-a'
        ? Promise.resolve(createRuntimeSnapshot('game-a'))
        : gameBRuntime.promise;
    });

    const { result, rerender } = renderHook(
      ({ gameId }: { gameId: string }) => useCollectionRuntime(gameId),
      { initialProps: { gameId: 'game-a' }, wrapper: createWrapper(queryClient) },
    );

    await waitFor(() =>
      expect(result.current.data?.current_objects[0]?.object_id).toBe('object-game-a'),
    );
    rerender({ gameId: 'game-b' });

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('get_collection_runtime_state', { gameId: 'game-b' }),
    );
    expect(result.current.data).toBeUndefined();
    expect(result.current.isPlaceholderData).toBe(false);
  });
});
