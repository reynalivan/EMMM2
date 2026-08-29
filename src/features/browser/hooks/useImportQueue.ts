import { commands } from '../../../core/tauri/bindings';
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
    const unlistenJob = listen<ImportJobUpdateEvent>('import:job-update', () => {
      void publishQueryScopes(queryClient, ['browserImportQueue']);
    });
    const unlistenBatch = listen<ImportJobUpdateEvent>('import:batch-update', () => {
      void publishQueryScopes(queryClient, ['browserImportQueue']);
    });

    return () => {
      void unlistenJob.then((fn) => fn());
      void unlistenBatch.then((fn) => fn());
    };
  }, [queryClient]);

  const skipMutation = useMutation({
    mutationFn: (batchId: string) => commands.cancelImportBatch(batchId),
    onSuccess: async () => publishQueryScopes(queryClient, ['browserImportQueue']),
  });

  return {
    jobs: query.data ?? [],
    cancelBatch: skipMutation.mutate,
  };
}
