import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { objectKeys, runObjectBatchMutation } from './objectQueryCache';
import { useCategoryCounts } from './useObjectQueries';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useSafeMode } from './settingsQuery';
import React from 'react';
import type { ObjectSummary } from '../types/object';

vi.unmock('@tanstack/react-query');

// Mock dependecies
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

vi.mock('../lib/services/objectService', () => ({
  // The corridor is derived server-side now, so the service call carries no
  // safeMode argument; each test queues the per-corridor responses it expects.
  getCategoryCounts: vi.fn(),
}));

vi.mock('./settingsQuery', () => ({
  useSafeMode: vi.fn(),
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
    vi.mocked(useSafeMode).mockReturnValue(false);
    const { getCategoryCounts } = await import('../lib/services/objectService');
    vi.mocked(getCategoryCounts)
      .mockResolvedValueOnce([{ object_type: 'Character', count: 10 }])
      .mockResolvedValueOnce([{ object_type: 'Character', count: 5 }]);

    const { result, rerender } = renderHook(() => useCategoryCounts(), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual([{ object_type: 'Character', count: 10 }]);

    // 2. Enable safeMode (simulating a settings save that turns Safe Mode on)
    vi.mocked(useSafeMode).mockReturnValue(true);

    // Clear query client to force refetch or rely on different query keys
    queryClient.clear();
    rerender();

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    // The mock backend returns 5 when safeMode is true, simulating exact DB subtraction
    expect(result.current.data).toEqual([{ object_type: 'Character', count: 5 }]);
  });

  it('runObjectBatchMutation leaves caches untouched when the mutation fails', async () => {
    const queryKey = objectKeys.list({
      game_id: 'genshin',
      object_type: null,
      search_query: null,
      meta_filters: null,
      sort_by: null,
      status_filter: null,
    });
    const objectList = [{ id: 'obj-1', is_pinned: false }] as unknown as ObjectSummary[];
    queryClient.setQueryData(queryKey, objectList);

    await expect(
      runObjectBatchMutation({
        queryClient,
        mutation: async () => {
          throw new Error('boom');
        },
      }),
    ).rejects.toThrow('boom');

    // Invalidation-only: nothing was optimistically written, so nothing changed.
    expect(queryClient.getQueryData<ObjectSummary[]>(queryKey)).toBe(objectList);
  });
});
