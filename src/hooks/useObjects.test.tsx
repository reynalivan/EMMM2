import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useCategoryCounts } from './useObjects';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useAppStore } from '../stores/useAppStore';
import React from 'react';

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
    useAppStore.setState({ safeMode: false });

    const { result, rerender } = renderHook(() => useCategoryCounts(), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    expect(result.current.data).toEqual([{ category: 'Character', count: 10 }]);

    // 2. Enable safeMode (simulating lock)
    useAppStore.setState({ safeMode: true });

    // Clear query client to force refetch or rely on different query keys
    queryClient.clear();
    rerender();

    await waitFor(() => expect(result.current.isSuccess).toBe(true));
    // The mock backend returns 5 when safeMode is true, simulating exact DB subtraction
    expect(result.current.data).toEqual([{ category: 'Character', count: 5 }]);
  });
});
