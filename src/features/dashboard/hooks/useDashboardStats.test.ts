import { renderHook, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useDashboardStats } from './useDashboardStats';
import { useAppStore } from '../../../stores/useAppStore';
import { invoke } from '@tauri-apps/api/core';
import { createWrapper } from '../../../testing/test-utils';
import type { DashboardPayload } from '../../../types/dashboard';

// Restore real @tanstack/react-query — the global setupTests stub
// replaces useQuery with a no-op, which prevents queryFn from running.
vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../../stores/useAppStore', () => ({
  useAppStore: vi.fn(),
}));

describe('useDashboardStats', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should fetch stats respecting safeMode=false', async () => {
    vi.mocked(useAppStore).mockImplementation(((selector: unknown) => {
      const fn = selector as (state: { safeMode: boolean }) => unknown;
      return fn({ safeMode: false });
    }) as unknown as typeof useAppStore);
    const mockPayload: DashboardPayload = {
      stats: {
        total_mods: 2,
        enabled_mods: 2,
        disabled_mods: 0,
        total_size_bytes: 100,
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

    expect(invoke).toHaveBeenCalledWith('get_dashboard_stats', { safeMode: false });
    expect(result.current.data).toEqual(mockPayload);
  });

  it('should fetch stats respecting safeMode=true', async () => {
    vi.mocked(useAppStore).mockImplementation(((selector: unknown) => {
      const fn = selector as (state: { safeMode: boolean }) => unknown;
      return fn({ safeMode: true });
    }) as unknown as typeof useAppStore);
    const mockPayload: DashboardPayload = {
      stats: {
        total_mods: 1,
        enabled_mods: 1,
        disabled_mods: 0,
        total_size_bytes: 50,
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

    expect(invoke).toHaveBeenCalledWith('get_dashboard_stats', { safeMode: true });
    expect(result.current.data).toEqual(mockPayload);
  });
});
