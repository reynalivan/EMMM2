import { commands } from '../../../lib/bindings';
import { listen } from '@tauri-apps/api/event';
import { useQueryClient, useQuery, useMutation } from '@tanstack/react-query';
import { useEffect } from 'react';
import type { ImportJobItem, ImportJobUpdateEvent } from '../types';
import { publishQueryScopes } from '../../runtime-sync/queryRefresh';

export const IMPORT_QUEUE_KEY = ['import-queue'] as const;

/** Fetches the import job queue and subscribes to real-time Tauri events. */
export function useImportQueue() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: IMPORT_QUEUE_KEY,
    // Refine the wire DTO's plain-string status to the frontend union once, here.
    queryFn: async () => (await commands.browserListImportQueue()) as ImportJobItem[],
    refetchOnWindowFocus: false,
  });

  useEffect(() => {
    // The event only says a job moved; the refetch brings the whole row back —
    // including jobs auto-import queued while the panel was open.
    const unlisten = listen<ImportJobUpdateEvent>('import:job-update', () => {
      void publishQueryScopes(queryClient, ['browserImportQueue']);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [queryClient]);

  const confirmMutation = useMutation({
    mutationFn: ({
      jobId,
      gameId,
      category,
      objectId,
    }: {
      jobId: string;
      gameId: string;
      category: string;
      objectId?: string | null;
    }) => commands.browserConfirmImport(jobId, gameId, category, objectId ?? null),
    onSuccess: async () => publishQueryScopes(queryClient, ['browserImportQueue']),
  });

  const skipMutation = useMutation({
    mutationFn: (jobId: string) => commands.browserCancelImport(jobId),
    onSuccess: async () => publishQueryScopes(queryClient, ['browserImportQueue']),
  });

  // Jobs pending user decision
  const needsReview = (query.data ?? []).filter((j: ImportJobItem) => j.status === 'needs_review');

  return {
    jobs: query.data ?? [],
    isLoading: query.isLoading,
    needsReview,
    confirmJob: confirmMutation.mutate,
    skipJob: skipMutation.mutate,
  };
}
