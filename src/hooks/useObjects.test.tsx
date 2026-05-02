import { describe, it, expect, vi, beforeEach } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import { objectKeys, patchObjectEnabledCount, runObjectBatchMutation } from './objectQueryCache';
import { useCategoryCounts } from './useObjectQueries';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useAppStore } from '../stores/useAppStore';
import React from 'react';
import type { ObjectSummary } from '../types/object';

vi.unmock('@tanstack/react-query');

// Mock dependecies
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

vi.mock('../lib/services/objectService', () => ({
  getCategoryCounts: vi.fn().mockImplementation((_gameId: string, safeMode: boolean) => {
    if (safeMode) {
      return Promise.resolve([{ category: 'Character', count: 5 }]);
    }
    return Promise.resolve([{ category: 'Character', count: 10 }]);
  }),
}));

vi.mock('./useActiveGame', () => ({
  useActiveGame: () => ({
    activeGame: {
      id: 'genshin',
    },
  }),
}));

const queryClient = new QueryClient();
const wrapper = ({ children }: { children: React.ReactNode }) => (
  <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
);

describe('useCategoryCounts (TC-30 Privacy & Safe Mode)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    queryClient.clear();
  });

  // TC-30-003: Verify object list counts decrement appropriately when entering safe mode.
  it('TC-30-003: Fetches filtered counts based on safeMode state', async () => {
    // 1. Render hook with safeMode = false
    act(() => {
      useAppStore.setState({ safeMode: false });
    });

    const { result, rerender } = renderHook(() => useCategoryCounts(), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual([{ category: 'Character', count: 10 }]);

    // 2. Enable safeMode (simulating lock)
    act(() => {
      useAppStore.setState({ safeMode: true });
    });

    // Clear query client to force refetch or rely on different query keys
    queryClient.clear();
    rerender();

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    // The mock backend returns 5 when safeMode is true, simulating exact DB subtraction
    expect(result.current.data).toEqual([{ category: 'Character', count: 5 }]);
  });

  it('optimistically patches enabled_count and clamps to valid bounds', () => {
    const objectList: ObjectSummary[] = [
      {
        id: 'obj-1',
        name: 'Alpha',
        folder_path: 'Alpha',
        object_type: 'Character',
        sub_category: null,
        status: 1,
        created_at: null,
        mod_count: 3,
        enabled_count: 1,
        thumbnail_path: null,
        is_pinned: false,
        is_auto_sync: false,
        is_object_disabled: false,
        has_naming_conflict: false,
        metadata: '{}',
        tags: '[]',
        hash_db: null,
        custom_skins: null,
        matched_entry_key: null,
        matched_alias_name: null,
        matched_confidence: null,
        matched_reason: null,
        matched_source: null,
        active_mod_paths: null,
      },
    ];

    queryClient.setQueryData(
      objectKeys.list({
        game_id: 'genshin',
        safe_mode: false,
        object_type: null,
        search_query: null,
        meta_filters: null,
        sort_by: null,
        status_filter: null,
      }),
      objectList,
    );

    patchObjectEnabledCount(queryClient, 'obj-1', 5);
    let patched = queryClient.getQueryData<ObjectSummary[]>(objectKeys.lists());
    expect(patched).toBeUndefined();

    patched = queryClient.getQueryData<ObjectSummary[]>(
      objectKeys.list({
        game_id: 'genshin',
        safe_mode: false,
        object_type: null,
        search_query: null,
        meta_filters: null,
        sort_by: null,
        status_filter: null,
      }),
    );
    expect(patched?.[0].enabled_count).toBe(3);

    patchObjectEnabledCount(queryClient, 'obj-1', -10);
    patched = queryClient.getQueryData<ObjectSummary[]>(
      objectKeys.list({
        game_id: 'genshin',
        safe_mode: false,
        object_type: null,
        search_query: null,
        meta_filters: null,
        sort_by: null,
        status_filter: null,
      }),
    );
    expect(patched?.[0].enabled_count).toBe(0);
  });

  it('runObjectBatchMutation restores cached rows on mutation failure', async () => {
    const queryKey = objectKeys.list({
      game_id: 'genshin',
      safe_mode: false,
      object_type: null,
      search_query: null,
      meta_filters: null,
      sort_by: null,
      status_filter: null,
    });
    const objectList: ObjectSummary[] = [
      {
        id: 'obj-1',
        name: 'Alpha',
        folder_path: 'Alpha',
        object_type: 'Character',
        sub_category: null,
        status: 1,
        created_at: null,
        mod_count: 3,
        enabled_count: 1,
        thumbnail_path: null,
        is_pinned: false,
        is_auto_sync: false,
        is_object_disabled: false,
        has_naming_conflict: false,
        metadata: '{}',
        tags: '[]',
        hash_db: null,
        custom_skins: null,
        matched_entry_key: null,
        matched_alias_name: null,
        matched_confidence: null,
        matched_reason: null,
        matched_source: null,
        active_mod_paths: null,
      },
    ];

    queryClient.setQueryData(queryKey, objectList);

    await expect(
      runObjectBatchMutation({
        queryClient,
        applyOptimisticUpdate: (object) => ({ ...object, is_pinned: true }),
        mutation: async () => {
          throw new Error('boom');
        },
      }),
    ).rejects.toThrow('boom');

    const restored = queryClient.getQueryData<ObjectSummary[]>(queryKey);
    expect(restored?.[0].is_pinned).toBe(false);
  });
});
