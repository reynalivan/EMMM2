import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useQueryClient, useQuery, useMutation } from '@tanstack/react-query';
import { useEffect } from 'react';
import type { ImportJobItem, ImportJobUpdateEvent } from '../types';

export const IMPORT_QUEUE_KEY = ['import-queue'] as const;

/** Fetches the import job queue and subscribes to real-time Tauri events. */
export function useImportQueue() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: IMPORT_QUEUE_KEY,
    queryFn: () => invoke<ImportJobItem[]>('import_get_queue'),
    refetchOnWindowFocus: false,
  });

  useEffect(() => {
    const unlisten = listen<ImportJobUpdateEvent>('import:job-update', (event) => {
      queryClient.setQueryData<ImportJobItem[]>(IMPORT_QUEUE_KEY, (old) => {
        if (!old) return old;
        return old.map((job) =>
          job.id === event.payload.job_id
            ? {
                ...job,
                status: event.payload.status,
                match_category: event.payload.category ?? job.match_category,
                match_object_id: event.payload.object_id ?? job.match_object_id,
                match_confidence: event.payload.confidence ?? job.match_confidence,
                match_reason: event.payload.reason ?? job.match_reason,
                placed_path: event.payload.placed_path ?? job.placed_path,
                error_msg: event.payload.error ?? job.error_msg,
              }
            : job,
        );
      });
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
    }) =>
      invoke('import_confirm_review', {
        jobId,
        gameId,
        category,
        objectId: objectId ?? null,
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: IMPORT_QUEUE_KEY }),
  });

  const skipMutation = useMutation({
    mutationFn: (jobId: string) => invoke('import_skip', { jobId }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: IMPORT_QUEUE_KEY }),
  });

  // Jobs pending user decision
  const needsReview = (query.data ?? []).filter((j) => j.status === 'needs_review');

  return {
    jobs: query.data ?? [],
    isLoading: query.isLoading,
    needsReview,
    confirmJob: confirmMutation.mutate,
    skipJob: skipMutation.mutate,
  };
}
