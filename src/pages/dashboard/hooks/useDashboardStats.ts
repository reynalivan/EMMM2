import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { DashboardPayload } from '../model/dashboard';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';
import { dashboardGateway } from '../api/dashboardGateway';

export const dashboardKeys = {
  all: ['dashboard-stats'] as const,
};

/**
 * TanStack Query hook for dashboard data.
 * Fetches all dashboard data in a single IPC call.
 * Cache: 30s staleTime per Epic 13 spec.
 */
export function useDashboardStats() {
  const queryClient = useQueryClient();

  const query = useQuery<DashboardPayload>({
    queryKey: dashboardKeys.all,
    queryFn: () => dashboardGateway.getDashboardStats(),
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
