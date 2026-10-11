import { renderHook, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useDashboardStats } from './useDashboardStats';
import { invoke } from '@tauri-apps/api/core';
import { createWrapper } from '../../../tests/testing/test-utils';
import type { DashboardPayload } from '../model/dashboard';
import { useAppStore } from '@/app/store';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createElement, type ReactNode } from 'react';

// Restore real @tanstack/react-query — the global setupTests stub
// replaces useQuery with a no-op, which prevents queryFn from running.
vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('useDashboardStats', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({ activeGameId: null, requestedGameId: null });
  });

  it('fetches unfiltered dashboard stats', async () => {
    const mockPayload: DashboardPayload = {
      stats: {
        total_mods: 2,
        enabled_mods: 2,
        disabled_mods: 0,
        total_games: 1,
        total_collections: 0,
      },
      duplicate_waste_bytes: 0,
      category_distribution: [],
      game_distribution: [],
      recent_mods: [],
    };
    vi.mocked(invoke).mockResolvedValue(mockPayload);

    const { result } = renderHook(() => useDashboardStats(), { wrapper: createWrapper });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(invoke).toHaveBeenCalledWith('get_dashboard_stats');
    expect(result.current.data).toEqual(mockPayload);
  });

  it('keeps provisional and authoritative dashboard snapshots in the selected game cache', async () => {
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    useAppStore.setState({ activeGameId: null, requestedGameId: 'startup-game' });
    vi.mocked(invoke).mockResolvedValue({ marker: 'startup-dashboard' });
    const { result } = renderHook(() => useDashboardStats(), {
      wrapper: ({ children }: { children: ReactNode }) =>
        createElement(QueryClientProvider, { client: queryClient }, children),
    });
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(queryClient.getQueryData(['dashboard-stats', 'startup-game'])).toEqual({
      marker: 'startup-dashboard',
    });
    expect(queryClient.getQueryData(['dashboard-stats'])).toBeUndefined();
  });
});
