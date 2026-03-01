/**
 * React hooks for Epic 9: Duplicate Scanner.
 * Follows the same pattern as useFolders.ts:
 * - Query key factory (dedupKeys)
 * - useQuery for data fetching
 * - useMutation for commands with proper cache invalidation
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { dedupService } from '../../../lib/services/dedupService';
import { folderKeys, trashKeys } from '../../../hooks/useFolders';
import { toast } from '../../../stores/useToastStore';
import type { DupScanReport, ResolutionRequest, DupScanEvent } from '../../../types/dedup';

/**
 * Query key factory for dedup cache management.
 * Ensures proper cache invalidation and refetch coordination.
 */
export const dedupKeys = {
  all: ['dedup'] as const,
  report: () => [...dedupKeys.all, 'report'] as const,
  events: () => [...dedupKeys.all, 'events'] as const,
};

/**
 * Fetch the last completed duplicate scan report.
 * Returns null if no scan has completed yet.
 *
 * Covers: Epic 9 report retrieval
 */
export function useDedupReport() {
  return useQuery<DupScanReport | null>({
    queryKey: dedupKeys.report(),
    queryFn: () => dedupService.getReport(),
    staleTime: 30_000, // Report valid for 30 seconds
    refetchOnWindowFocus: false,
  });
}

/**
 * Start a duplicate scan with real-time progress streaming.
 * Emits DupScanEvent variants as scan progresses.
 *
 * Invalidates the report cache after scan completes.
 *
 * Covers: Epic 9 scan initiation
 */
export function useStartDedupScan() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: {
      gameId: string;
      modsRoot: string;
      onEvent: (event: DupScanEvent) => void;
    }) => dedupService.startDedupScan(params.gameId, params.modsRoot, params.onEvent),

    onSuccess: () => {
      // Refresh report after scan completes
      queryClient.invalidateQueries({ queryKey: dedupKeys.report() });
    },

    onError: (error) => {
      toast.error(`Scan failed: ${String(error)}`);
    },
  });
}

/**
 * Cancel the currently running duplicate scan.
 * Safe to call if no scan is running.
 *
 * Covers: Epic 9 scan cancellation
 */
export function useCancelDedupScan() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => dedupService.cancelDedupScan(),

    onSuccess: () => {
      // Refresh report after cancellation
      queryClient.invalidateQueries({ queryKey: dedupKeys.report() });
      toast.info('Scan cancelled');
    },

    onError: (error) => {
      toast.error(`Cancel failed: ${String(error)}`);
    },
  });
}

/**
 * Resolve duplicate groups with batch actions.
 * Executes delete/keep/whitelist operations and updates folder/trash caches.
 *
 * Critical invalidation:
 * - folderKeys.all: Folders may have been deleted
 * - trashKeys.all: New items may be in trash
 * - dedupKeys.all: Report is stale after resolution
 *
 * Covers: Epic 9 resolution execution
 */
export function useResolveDuplicates() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: { requests: ResolutionRequest[]; gameId: string }) =>
      dedupService.resolveBatch(params.requests, params.gameId),

    onSuccess: (summary) => {
      // Invalidate all caches affected by resolution
      queryClient.invalidateQueries({ queryKey: folderKeys.all });
      queryClient.invalidateQueries({ queryKey: trashKeys.all });
      queryClient.invalidateQueries({ queryKey: dedupKeys.all });

      // Show resolution summary
      if (summary.failed === 0) {
        toast.success(`Resolved ${summary.successful}/${summary.total} duplicates`);
      } else {
        toast.warning(
          `Resolved ${summary.successful}/${summary.total} duplicates (${summary.failed} failed)`,
        );
      }
    },

    onError: (error) => {
      toast.error(`Resolution failed: ${String(error)}`);
    },
  });
}
