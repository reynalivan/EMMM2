import React from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createWrapper } from '../../../tests/testing/test-utils';
import { collectionRuntimeKeys } from '@/entities/collection';
import {
  useApplyCollection,
  useApplyCollectionPreview,
  useCollections,
  useRestoreLastChanges,
} from './useCollections';
import type { CollectionRuntimeSnapshot } from '@/entities/collection';
import { useAppStore } from '@/app/store';
import { toast } from '@/shared/ui/toast';

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

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
    withAction: vi.fn(),
  },
}));

function createMutationWrapper(queryClient: QueryClient) {
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
        mod_count: 5,
      },
    ]);

    const { result } = renderHook(() => useCollections('g-1'), {
      wrapper: createWrapper,
    });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(invoke).toHaveBeenCalledWith('list_collections', { gameId: 'g-1' });
    expect(result.current.data?.[0].name).toBe('Abyss Team');
  });

  it('does not retain game A collection IDs while game B is still loading', async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const gameBCollections = createDeferred<unknown>();
    vi.mocked(invoke).mockImplementation((command, args) => {
      if (command !== 'list_collections') {
        throw new Error(`Unexpected invoke command: ${command}`);
      }

      const { gameId } = args as { gameId: string };
      if (gameId === 'game-a') {
        return Promise.resolve([
          {
            id: 'collection-a',
            name: 'Game A only',
            is_safe: true,
            is_safety_classified: true,
            is_active: false,
            signature: 'signature-a',
            updated_at: '2026-08-28T00:00:00Z',
            mod_count: 1,
          },
        ]);
      }

      return gameBCollections.promise;
    });

    const { result, rerender } = renderHook(
      ({ gameId }: { gameId: string }) => useCollections(gameId),
      { initialProps: { gameId: 'game-a' }, wrapper: createMutationWrapper(queryClient) },
    );

    await waitFor(() => expect(result.current.data?.[0]?.id).toBe('collection-a'));
    rerender({ gameId: 'game-b' });

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('list_collections', { gameId: 'game-b' }),
    );
    expect(result.current.data).toBeUndefined();
    expect(result.current.isPlaceholderData).toBe(false);
  });

  it('apply invalidates runtime state and applies backend path rewrites', async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });
    const wrapper = createMutationWrapper(queryClient);

    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'apply_collection') {
        return {
          mods_enabled: 1,
          mods_disabled: 0,
          warnings: [],
          final_state_name: 'Backend Runtime',
          partial_apply: false,
          skipped_missing_paths: [],
          sync_warning: {
            kind: 'ReconcileFailed',
            message: 'Projection refresh is pending',
          },
          runtime_path_rewrites: [
            {
              old_path: 'E:/Mods/ALBEDO/Variant',
              new_path: 'E:/Mods/ALBEDO/DISABLED Variant',
            },
          ],
        };
      }

      if (command === 'list_collections') {
        return [];
      }

      if (command === 'get_collection_runtime_state') {
        return {
          game_id: 'g-1',
          active_collection_id: 'c-1',
          active_collection_name: 'Backend Runtime',
          current_signature: 'backend-sig',
          is_dirty: false,
          runtime_status: 'clean',
          is_safe: true,
          is_safety_classified: true,
          missing_count: 0,
          last_changes: null,
          current_mods: [],
          current_objects: [],
          current_tree_nodes: [],
          projected_state: createProjectedState(),
        } satisfies CollectionRuntimeSnapshot;
      }

      throw new Error(`Unexpected invoke command: ${command}`);
    });

    queryClient.setQueryData<CollectionRuntimeSnapshot>(collectionRuntimeKeys.state('g-1'), {
      game_id: 'g-1',
      active_collection_id: null,
      active_collection_name: 'Old Snapshot',
      current_signature: 'old',
      is_dirty: false,
      runtime_status: 'clean',
      is_safe: true,
      is_safety_classified: true,
      missing_count: 0,
      last_changes: null,
      current_mods: [],
      current_objects: [],
      current_tree_nodes: [],
      projected_state: createProjectedState(),
    });
    useAppStore.setState({
      selectedModPath: 'E:/Mods/ALBEDO/Variant',
      gridSelection: new Set(['E:/Mods/ALBEDO/Variant']),
    });

    const { result } = renderHook(() => useApplyCollection(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync({
        collectionId: 'c-1',
        gameId: 'g-1',
      });
    });

    // Invalidation-only: the stale snapshot is marked for refetch, never patched.
    expect(queryClient.getQueryState(collectionRuntimeKeys.state('g-1'))?.isInvalidated).toBe(true);
    expect(
      queryClient.getQueryData<CollectionRuntimeSnapshot>(collectionRuntimeKeys.state('g-1')),
    ).toMatchObject({
      active_collection_name: 'Old Snapshot',
    });
    expect(useAppStore.getState().selectedModPath).toBe('E:/Mods/ALBEDO/DISABLED Variant');
    expect(useAppStore.getState().gridSelection.has('E:/Mods/ALBEDO/DISABLED Variant')).toBe(true);
    expect(toast.warning).toHaveBeenCalledWith(
      'Disk changes were applied, but runtime refresh is still pending.',
      7000,
    );
  });

  it('restore last changes applies backend path rewrites to workspace selection', async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });
    const wrapper = createMutationWrapper(queryClient);
    vi.mocked(invoke).mockImplementation(async (command: string) => {
      if (command === 'restore_last_changes') {
        return {
          mods_enabled: 0,
          mods_disabled: 1,
          warnings: [],
          final_state_name: 'Previous Runtime',
          partial_apply: false,
          skipped_missing_paths: [],
          runtime_path_rewrites: [
            {
              old_path: 'E:/Mods/ALBEDO/DISABLED Variant',
              new_path: 'E:/Mods/ALBEDO/Variant',
            },
          ],
        };
      }

      throw new Error(`Unexpected invoke command: ${command}`);
    });
    useAppStore.setState({
      selectedModPath: 'E:/Mods/ALBEDO/DISABLED Variant',
      gridSelection: new Set(['E:/Mods/ALBEDO/DISABLED Variant']),
    });

    const { result } = renderHook(() => useRestoreLastChanges(), { wrapper });

    await act(async () => {
      await result.current.mutateAsync('g-1');
    });

    expect(useAppStore.getState().selectedModPath).toBe('E:/Mods/ALBEDO/Variant');
    expect(useAppStore.getState().gridSelection.has('E:/Mods/ALBEDO/Variant')).toBe(true);
  });

  it('refetches apply preview when game id changes', async () => {
    vi.mocked(invoke).mockResolvedValue({
      collection_name: 'Preset',
      current_tree_nodes: [],
      target_tree_nodes: [],
      current_state_name: null,
      current_state_is_unsaved: false,
      current_projected_state: createProjectedState(),
      target_projected_state: createProjectedState(),
    });

    const { rerender } = renderHook(
      ({ gameId }: { gameId: string }) => useApplyCollectionPreview(gameId, 'c-1'),
      {
        initialProps: { gameId: 'g-1' },
        wrapper: createWrapper,
      },
    );

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('preview_apply_collection', {
        gameId: 'g-1',
        collectionId: 'c-1',
      }),
    );

    vi.mocked(invoke).mockClear();
    rerender({ gameId: 'g-2' });

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('preview_apply_collection', {
        gameId: 'g-2',
        collectionId: 'c-1',
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
