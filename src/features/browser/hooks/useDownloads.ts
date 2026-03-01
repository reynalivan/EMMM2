import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useQueryClient, useQuery, useMutation } from '@tanstack/react-query';
import { useEffect } from 'react';
import type { BrowserDownloadItem, DownloadStatusEvent, DownloadProgressEvent } from '../types';

export const DOWNLOADS_QUERY_KEY = ['browser-downloads'] as const;

/** Fetches all browser downloads and subscribes to real-time Tauri events. */
export function useDownloads() {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: DOWNLOADS_QUERY_KEY,
    queryFn: () => invoke<BrowserDownloadItem[]>('browser_list_downloads'),
    refetchOnWindowFocus: false,
  });

  useEffect(() => {
    // Status changes (finished, failed, canceled, imported)
    const unlistenStatus = listen<DownloadStatusEvent>('browser:download-status', (event) => {
      queryClient.setQueryData<BrowserDownloadItem[]>(DOWNLOADS_QUERY_KEY, (old) => {
        if (!old) return old;
        return old.map((d) =>
          d.id === event.payload.id
            ? {
                ...d,
                status: event.payload.status,
                file_path:
                  event.payload.file_path !== undefined
                    ? (event.payload.file_path ?? d.file_path)
                    : d.file_path,
              }
            : d,
        );
      });
    });

    // Progress updates (bytes_received / bytes_total)
    const unlistenProgress = listen<DownloadProgressEvent>('browser:download-progress', (event) => {
      queryClient.setQueryData<BrowserDownloadItem[]>(DOWNLOADS_QUERY_KEY, (old) => {
        if (!old) return old;
        return old.map((d) =>
          d.id === event.payload.id
            ? {
                ...d,
                status: 'in_progress' as const,
                bytes_received: event.payload.bytes_received,
                bytes_total: event.payload.bytes_total,
              }
            : d,
        );
      });
    });

    return () => {
      unlistenStatus.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
    };
  }, [queryClient]);

  // --- Mutations ---

  const deleteMutation = useMutation({
    mutationFn: ({ id, deleteFile }: { id: string; deleteFile: boolean }) =>
      invoke('browser_delete_download', { id, deleteFile }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: DOWNLOADS_QUERY_KEY }),
  });

  const cancelMutation = useMutation({
    mutationFn: (id: string) => invoke('browser_cancel_download', { id, deleteFile: false }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: DOWNLOADS_QUERY_KEY }),
  });

  const clearImportedMutation = useMutation({
    mutationFn: () => invoke('browser_clear_imported'),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: DOWNLOADS_QUERY_KEY }),
  });

  return {
    downloads: query.data ?? [],
    isLoading: query.isLoading,
    isError: query.isError,
    deleteDownload: deleteMutation.mutate,
    cancelDownload: cancelMutation.mutate,
    clearImported: clearImportedMutation.mutate,
    finishedCount: (query.data ?? []).filter((d) => d.status === 'finished').length,
  };
}
