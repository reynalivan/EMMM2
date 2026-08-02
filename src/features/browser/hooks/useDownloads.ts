import { commands } from '../../../lib/bindings';
import { listen } from '@tauri-apps/api/event';
import { useQueryClient, useQuery, useMutation } from '@tanstack/react-query';
import { useEffect } from 'react';
import type { BrowserDownloadItem, DownloadStatusEvent, DownloadProgressEvent } from '../types';
import { publishQueryScopes } from '../../runtime-sync/queryRefresh';

export const DOWNLOADS_QUERY_KEY = ['browser-downloads'] as const;

/** Fetches all browser downloads and subscribes to real-time Tauri events. */
export function useDownloads() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: DOWNLOADS_QUERY_KEY,
    // Refine the wire DTO's plain-string status to the frontend union once, here.
    queryFn: async () => (await commands.browserListDownloads()) as BrowserDownloadItem[],
    refetchOnWindowFocus: false,
  });

  useEffect(() => {
    // Status changes (finished, failed, canceled, imported). The backend commits the
    // row before emitting, so a refetch always reads the new state.
    const unlistenStatus = listen<DownloadStatusEvent>('browser:download-status', () => {
      void publishQueryScopes(queryClient, ['browserDownloads']);
    });

    // ponytail: byte counters are the one thing still patched into the cache.
    // download_handler.rs emits progress at ~10Hz per active download; invalidating
    // on each would refetch the whole list dozens of times a second. Every other
    // field on the row still comes from a refetch driven by the status event.
    const unlistenProgress = listen<DownloadProgressEvent>('browser:download-progress', (event) => {
      queryClient.setQueryData<BrowserDownloadItem[]>(DOWNLOADS_QUERY_KEY, (old) =>
        old?.map((d) =>
          d.id === event.payload.id
            ? {
                ...d,
                status: 'in_progress' as const,
                bytes_received: event.payload.bytes_received,
                bytes_total: event.payload.bytes_total,
              }
            : d,
        ),
      );
    });

    return () => {
      unlistenStatus.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
    };
  }, [queryClient]);

  // --- Mutations ---

  const deleteMutation = useMutation({
    mutationFn: ({ id, deleteFile }: { id: string; deleteFile: boolean }) =>
      commands.browserDeleteDownload(id, deleteFile),
    onSuccess: async () => publishQueryScopes(queryClient, ['browserDownloads']),
  });

  const cancelMutation = useMutation({
    mutationFn: (id: string) => commands.browserCancelDownload(id, false),
    onSuccess: async () => publishQueryScopes(queryClient, ['browserDownloads']),
  });

  const clearImportedMutation = useMutation({
    mutationFn: () => commands.browserClearImported(),
    onSuccess: async () => publishQueryScopes(queryClient, ['browserDownloads']),
  });

  return {
    downloads: query.data ?? [],
    deleteDownload: deleteMutation.mutate,
    cancelDownload: cancelMutation.mutate,
    clearImported: clearImportedMutation.mutate,
    finishedCount: (query.data ?? []).filter((d: BrowserDownloadItem) => d.status === 'finished')
      .length,
  };
}
