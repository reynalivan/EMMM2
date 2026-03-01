import { renderHook, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useDashboardStats } from './useDashboardStats';
import { useAppStore } from '../../../stores/useAppStore';
import { invoke } from '@tauri-apps/api/core';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import React from 'react';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../../stores/useAppStore', () => ({
  useAppStore: vi.fn(),
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return ({ children }: { children: React.ReactNode }) =>
    React.createElement(QueryClientProvider, { client: queryClient }, children);
};

describe('useDashboardStats', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should fetch stats respecting safeMode=false', async () => {
    vi.mocked(useAppStore).mockImplementation(((selector: unknown) => {
      const fn = selector as (state: { safeMode: boolean }) => unknown;
      return fn({ safeMode: false });
    }) as unknown as typeof useAppStore);
    const mockPayload = { games: 2, storage: 100 };
    vi.mocked(invoke).mockResolvedValue(mockPayload);

    const { result } = renderHook(() => useDashboardStats(), { wrapper: createWrapper() });

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
    const mockPayload = { games: 1, storage: 50 };
    vi.mocked(invoke).mockResolvedValue(mockPayload);

    const { result } = renderHook(() => useDashboardStats(), { wrapper: createWrapper() });

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(invoke).toHaveBeenCalledWith('get_dashboard_stats', { safeMode: true });
    expect(result.current.data).toEqual(mockPayload);
  });
});
