import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { objectKeys, runObjectBatchMutation } from './objectQueryCache';
import { useCategoryCounts } from './useObjectQueries';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';
import type { ObjectSummary } from '../types/object';

vi.unmock('@tanstack/react-query');

// Mock dependecies
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

vi.mock('../lib/services/objectService', () => ({
  // Safety filtering is view-only, so the service call carries no filter
  // argument; each test queues the complete responses it expects.
  getCategoryCounts: vi.fn(),
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

describe('useCategoryCounts', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    queryClient.clear();
  });

  it('fetches unfiltered category counts', async () => {
    const { getCategoryCounts } = await import('../lib/services/objectService');
    vi.mocked(getCategoryCounts).mockResolvedValueOnce([{ object_type: 'Character', count: 10 }]);

    const { result } = renderHook(() => useCategoryCounts(), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual([{ object_type: 'Character', count: 10 }]);
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
