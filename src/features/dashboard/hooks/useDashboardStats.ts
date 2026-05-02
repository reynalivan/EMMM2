import { useQuery, useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../lib/bindings';
import type { DashboardPayload } from '../../../types/dashboard';
import { useAppStore } from '../../../stores/useAppStore';
import { publishQueryScopes } from '../../runtime-sync/queryRefresh';

export const dashboardKeys = {
  all: ['dashboard-stats'] as const,
};

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
    queryKey: [...dashboardKeys.all, safeMode],
    queryFn: () => commands.getDashboardStats({ safeMode }),
    staleTime: 30_000,
  });

  const refresh = () => {
    void publishQueryScopes(queryClient, ['dashboard']);
  };

  return {
    data: query.data,
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
    refresh,
  };
}
