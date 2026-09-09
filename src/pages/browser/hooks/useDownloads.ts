import { commands } from '@/shared/api/tauri/bindings';
import { listen } from '@tauri-apps/api/event';
import { useQueryClient, useQuery, useMutation } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import type { BrowserDownloadItem, DownloadStatusEvent, DownloadProgressEvent } from '../types';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';
import { toast } from '@/shared/ui/toast';
import { useTranslation } from 'react-i18next';

export const DOWNLOADS_QUERY_KEY = ['browser-downloads'] as const;

interface UseDownloadsOptions {
  /** BrowserPage is the sole toast owner; nested download views stay silent. */
  showFeedback?: boolean;
  onOpenDownloads?: () => void;
}

const progressByDownloadId = new Map<string, DownloadProgressEvent>();

function applyLatestProgress(download: BrowserDownloadItem): BrowserDownloadItem {
  const progress = progressByDownloadId.get(download.id);
  if (!progress) return download;

  return {
    ...download,
    status: 'in_progress',
    bytes_received: progress.bytes_received,
    bytes_total: progress.bytes_total,
  };
}

/** Fetches all browser downloads and subscribes to real-time Tauri events. */
export function useDownloads({ showFeedback = false, onOpenDownloads }: UseDownloadsOptions = {}) {
  const queryClient = useQueryClient();
  const { t } = useTranslation(['browser']);
  const announcedStatuses = useRef(new Set<string>());

  const query = useQuery({
    queryKey: DOWNLOADS_QUERY_KEY,
    // Refine the wire DTO's plain-string status to the frontend union once, here.
    queryFn: async () =>
      ((await commands.browserListDownloads()) as BrowserDownloadItem[]).map(applyLatestProgress),
    refetchOnWindowFocus: false,
  });

  const retryMutation = useMutation({
    mutationFn: (id: string) => commands.browserRetryDownload(id),
    onSuccess: async () => publishQueryScopes(queryClient, ['browserDownloads']),
    onError: () => toast.error(t('downloads.feedback.retry_failed')),
  });
  const retryDownload = retryMutation.mutate;

  useEffect(() => {
    // Status changes are emitted after the backend persists the row. Upsert the optional
    // snapshot first so a just-queued row can render before a following progress event.
    const unlistenStatus = listen<DownloadStatusEvent>('browser:download-status', (event) => {
      const { download, file_path: filePath, filename, id, status } = event.payload;

      queryClient.setQueryData<BrowserDownloadItem[]>(DOWNLOADS_QUERY_KEY, (old) => {
        if (download) {
          const next = applyLatestProgress(download);
          const existing = old ?? [];
          const index = existing.findIndex((item) => item.id === next.id);
          if (index === -1) return [next, ...existing];

          return existing.map((item) => (item.id === next.id ? next : item));
        }

        return old?.map((item) =>
          item.id === id
            ? {
                ...item,
                status,
                file_path: filePath ?? item.file_path,
              }
            : item,
        );
      });

      void publishQueryScopes(queryClient, ['browserDownloads']);

      if (!showFeedback || status === 'in_progress' || status === 'requested') return;

      const announcementKey = `${id}:${status}`;
      if (announcedStatuses.current.has(announcementKey)) return;
      announcedStatuses.current.add(announcementKey);

      if (!onOpenDownloads) return;

      const name = filename ?? download?.filename ?? t('downloads.feedback.this_file');
      const openDownloadsAction = {
        label: t('downloads.view_detail'),
        onClick: onOpenDownloads,
      };

      if (status === 'finished') {
        toast.withAction(
          'success',
          t('downloads.feedback.finished', { filename: name }),
          openDownloadsAction,
        );
      } else if (status === 'failed') {
        toast.withAction('error', t('downloads.feedback.failed', { filename: name }), {
          label: t('downloads.retry'),
          onClick: () => retryDownload(id),
        });
      } else if (status === 'canceled') {
        toast.withAction(
          'info',
          t('downloads.feedback.canceled', { filename: name }),
          openDownloadsAction,
        );
      }
    });

    // ponytail: byte counters are the one thing still patched into the cache.
    // download_handler.rs emits progress at ~10Hz per active download; invalidating
    // on each would refetch the whole list dozens of times a second. Every other
    // field on the row still comes from a refetch driven by the status event.
    const unlistenProgress = listen<DownloadProgressEvent>('browser:download-progress', (event) => {
      progressByDownloadId.set(event.payload.id, event.payload);
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
  }, [onOpenDownloads, queryClient, retryDownload, showFeedback, t]);

  // --- Mutations ---

  const deleteMutation = useMutation({
    mutationFn: ({ id, deleteFile }: { id: string; deleteFile: boolean }) =>
      commands.browserDeleteDownload(id, deleteFile),
    onSuccess: async () => publishQueryScopes(queryClient, ['browserDownloads']),
    onError: () => toast.error(t('downloads.feedback.delete_failed')),
  });

  const cancelMutation = useMutation({
    mutationFn: (id: string) => commands.browserCancelDownload(id, false),
    onSuccess: async () => publishQueryScopes(queryClient, ['browserDownloads']),
    onError: () => toast.error(t('downloads.feedback.cancel_failed')),
  });

  const clearImportedMutation = useMutation({
    mutationFn: () => commands.browserClearImported(),
    onSuccess: async () => publishQueryScopes(queryClient, ['browserDownloads']),
    onError: () => toast.error(t('downloads.feedback.clear_failed')),
  });

  const refreshDownloads = async () => {
    const result = await query.refetch();
    if (result.error) {
      toast.error(t('downloads.feedback.refresh_failed'));
    }
  };

  const downloads = query.data ?? [];

  return {
    downloads,
    deleteDownload: deleteMutation.mutate,
    cancelDownload: cancelMutation.mutate,
    clearImported: clearImportedMutation.mutate,
    retryDownload,
    refreshDownloads,
    isRefreshing: query.isRefetching,
    activeCount: downloads.filter((download) => download.status === 'in_progress').length,
    queuedCount: downloads.filter((download) => download.status === 'requested').length,
  };
}
