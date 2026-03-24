/**
 * React hooks for Epic 9: Duplicate Scanner.
 * Follows the same pattern as useFolders.ts:
 * - Query key factory (dedupKeys)
 * - useQuery for data fetching
 * - useMutation for commands with proper cache invalidation
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import i18next from 'i18next';
import { dedupService } from '../../../lib/services/dedupService';
import { folderKeys, trashKeys } from '../../../hooks/useFolders';
import { toast } from '../../../stores/useToastStore';
import type { DupScanReport, ResolutionRequest, DupScanEvent } from '../../../types/scanner';

/**
 * Query key factory for dedup cache management.
 * Ensures proper cache invalidation and refetch coordination.
 */
export const dedupKeys = {
  all: ['dedup'] as const,
  report: (pin?: string) => [...dedupKeys.all, 'report', pin || 'none'] as const,
  ignored: (gameId: string) => [...dedupKeys.all, 'ignored', gameId] as const,
  events: () => [...dedupKeys.all, 'events'] as const,
};

/**
 * Fetch detailed list of ignored (whitelisted) duplicate pairs.
 */
export function useIgnoredPairs(gameId: string) {
  return useQuery({
    queryKey: dedupKeys.ignored(gameId),
    queryFn: () => dedupService.getIgnoredPairs(gameId),
    staleTime: 60_000,
    enabled: !!gameId,
  });
}

/**
 * Remove a pair from the duplicate whitelist (recover it).
 */
export function useRemoveIgnoredPair() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (entryId: string) => dedupService.removeIgnoredPair(entryId),
    onSuccess: (_, _entryId) => {
      queryClient.invalidateQueries({ queryKey: dedupKeys.all });
      toast.success(i18next.t('scanner:dedup.toast.recover_success'));
    },
    onError: (error) => {
      toast.error(i18next.t('scanner:dedup.toast.recover_failed', { error: String(error) }));
    },
  });
}

/**
 * Fetch the last completed duplicate scan report.
 * Returns null if no scan has completed yet.
 *
 * Covers: Epic 9 report retrieval
 */
export function useDedupReport(pin?: string) {
  return useQuery<DupScanReport | null>({
    queryKey: dedupKeys.report(pin),
    queryFn: () => dedupService.getReport(pin),
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
      toast.error(i18next.t('scanner:dedup.toast.scan_failed', { error: String(error) }));
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
      toast.info(i18next.t('scanner:dedup.toast.scan_cancelled'));
    },

    onError: (error) => {
      toast.error(i18next.t('scanner:dedup.toast.cancel_failed', { error: String(error) }));
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
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['conflicts'] });

      // Show resolution summary
      if (summary.failed === 0) {
        toast.success(
          i18next.t('scanner:dedup.toast.resolve_success', {
            successful: summary.successful,
            total: summary.total,
          }),
        );
      } else {
        toast.warning(
          i18next.t('scanner:dedup.toast.resolve_warning', {
            successful: summary.successful,
            total: summary.total,
            failed: summary.failed,
          }),
        );
      }
    },

    onError: (error) => {
      toast.error(i18next.t('scanner:dedup.toast.resolve_failed', { error: String(error) }));
    },
  });
}
