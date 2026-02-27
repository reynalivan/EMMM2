import { useQuery, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../../stores/useAppStore';
import type { DashboardPayload } from '../../../types/dashboard';

const DASHBOARD_QUERY_KEY = ['dashboard-stats'] as const;

/**
 * TanStack Query hook for dashboard data.
 * Fetches all dashboard data in a single IPC call.
 * Respects Safe Mode from appStore (filters NSFW when active).
 * Cache: 30s staleTime per Epic 13 spec.
 */
export function useDashboardStats() {
  const safeMode = useAppStore((s) => s.safeMode);
  const queryClient = useQueryClient();

  const query = useQuery<DashboardPayload>({
    queryKey: [...DASHBOARD_QUERY_KEY, safeMode],
    queryFn: () => invoke<DashboardPayload>('get_dashboard_stats', { safeMode }),
    staleTime: 30_000,
  });

  const refresh = () => {
    queryClient.invalidateQueries({ queryKey: DASHBOARD_QUERY_KEY });
  };

  return {
    data: query.data,
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
    refresh,
  };
}
