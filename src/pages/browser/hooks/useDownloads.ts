import { commands } from '@/shared/api/tauri/bindings';
import { listen } from '@tauri-apps/api/event';
import { useQueryClient, useQuery, useMutation } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import type { BrowserDownloadItem, DownloadStatusEvent, DownloadProgressEvent } from '../types';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';
import { toast } from '@/shared/ui/toast';
import { useTranslation } from 'react-i18next';
import { getDownloadFailureMessageKey } from '../downloadPresentation';
import { isDemoMode } from '@/shared/lib/appMode';

export const downloadsQueryKey = (gameId: string | null) => ['browser-downloads', gameId] as const;

interface UseDownloadsOptions {
  /** BrowserPage is the sole toast owner; nested download views stay silent. */
  showFeedback?: boolean;
  onOpenDownloads?: () => void;
  /** Exactly one mounted surface owns native event subscriptions for a game. */
  subscribe?: boolean;
}

const progressByDownloadId = new Map<string, DownloadProgressEvent>();

function isTerminalStatus(status: BrowserDownloadItem['status']): boolean {
  return (
    status === 'finished' || status === 'failed' || status === 'canceled' || status === 'imported'
  );
}

export function mergeDownloadProgress(
  download: BrowserDownloadItem,
  progress: DownloadProgressEvent,
): BrowserDownloadItem {
  if (isTerminalStatus(download.status)) {
    return download;
  }

  return {
    ...download,
    status: 'in_progress',
    bytes_received: progress.bytes_received,
    bytes_total: progress.bytes_total,
  };
}

function applyLatestProgress(download: BrowserDownloadItem): BrowserDownloadItem {
  if (isTerminalStatus(download.status)) {
    progressByDownloadId.delete(download.id);
    return download;
  }

  const progress = progressByDownloadId.get(download.id);
  if (!progress) return download;

  return mergeDownloadProgress(download, progress);
}

/** Fetches all browser downloads and subscribes to real-time Tauri events. */
export function useDownloads(
  gameId: string | null,
  { showFeedback = false, onOpenDownloads, subscribe = true }: UseDownloadsOptions = {},
) {
  const queryClient = useQueryClient();
  const { t } = useTranslation(['browser']);
  const announcedStatuses = useRef(new Set<string>());

  const query = useQuery({
    queryKey: downloadsQueryKey(gameId),
    // Refine the wire DTO's plain-string status to the frontend union once, here.
    queryFn: async () =>
      ((await commands.browserListDownloads(gameId!)) as BrowserDownloadItem[]).map(
        applyLatestProgress,
      ),
    enabled: Boolean(gameId),
    refetchOnWindowFocus: false,
  });

  const retryMutation = useMutation({
    mutationFn: (id: string) => commands.browserRetryDownload(id),
    onSuccess: async () => publishQueryScopes(queryClient, ['browserDownloads']),
    onError: () => toast.error(t('downloads.feedback.retry_failed')),
  });
  const retryDownload = retryMutation.mutate;

  useEffect(() => {
    if (isDemoMode || !subscribe) {
      return;
    }

    // Status changes are emitted after the backend persists the row. Upsert the optional
    // snapshot first so a just-queued row can render before a following progress event.
    const unlistenStatus = listen<DownloadStatusEvent>('browser:download-status', (event) => {
      const {
        download,
        error_msg: errorMessage,
        file_path: filePath,
        filename,
        can_resume: canResume,
        id,
        status,
      } = event.payload;
      if (isTerminalStatus(status)) {
        progressByDownloadId.delete(id);
      }

      if (download?.game_id && download.game_id !== gameId) return;

      queryClient.setQueryData<BrowserDownloadItem[]>(downloadsQueryKey(gameId), (old) => {
        if (download) {
          const next = applyLatestProgress({
            ...download,
            status,
            file_path: filePath ?? download.file_path,
            error_msg: errorMessage ?? download.error_msg,
            can_resume: canResume ?? download.can_resume,
          });
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
                error_msg: errorMessage ?? item.error_msg,
                can_resume: canResume ?? item.can_resume,
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
        toast.withAction(
          'error',
          t(getDownloadFailureMessageKey(errorMessage ?? download?.error_msg ?? null)),
          {
            label: t('downloads.retry'),
            onClick: () => retryDownload(id),
          },
        );
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
      queryClient.setQueryData<BrowserDownloadItem[]>(downloadsQueryKey(gameId), (old) =>
        old?.map((download) => {
          if (download.id !== event.payload.id) {
            return download;
          }
          if (isTerminalStatus(download.status)) {
            progressByDownloadId.delete(download.id);
            return download;
          }
          return mergeDownloadProgress(download, event.payload);
        }),
      );
    });

    return () => {
      unlistenStatus.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
    };
  }, [gameId, onOpenDownloads, queryClient, retryDownload, showFeedback, subscribe, t]);

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

  const pauseMutation = useMutation({
    mutationFn: (id: string) => commands.browserPauseDownload(id),
    onError: () => toast.error(t('downloads.feedback.pause_failed')),
  });

  const resumeMutation = useMutation({
    mutationFn: (id: string) => commands.browserResumeDownload(id),
    onError: () => toast.error(t('downloads.feedback.resume_failed')),
  });

  const refreshLinkMutation = useMutation({
    mutationFn: (id: string) => commands.browserRefreshDownloadLink(id),
    onError: () => toast.error(t('downloads.feedback.refresh_link_failed')),
  });

  const openSourceMutation = useMutation({
    mutationFn: (id: string) => commands.browserOpenDownloadSource(id),
    onError: () => toast.error(t('downloads.feedback.open_source_failed')),
  });

  const renameMutation = useMutation({
    mutationFn: ({ id, filename }: { id: string; filename: string }) =>
      commands.browserRenameDownload(id, filename),
    onSuccess: async () => publishQueryScopes(queryClient, ['browserDownloads']),
    onError: () => toast.error(t('downloads.feedback.rename_failed')),
  });

  const openFileMutation = useMutation({
    mutationFn: (id: string) => commands.browserOpenDownloadFile(id),
    onError: () => toast.error(t('downloads.feedback.open_file_failed')),
  });

  const openLocationMutation = useMutation({
    mutationFn: (id: string) => commands.browserOpenDownloadLocation(id),
    onError: () => toast.error(t('downloads.feedback.open_location_failed')),
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
    pauseDownload: pauseMutation.mutate,
    resumeDownload: resumeMutation.mutate,
    refreshDownloadLink: refreshLinkMutation.mutate,
    openDownloadSource: openSourceMutation.mutate,
    renameDownload: renameMutation.mutateAsync,
    openDownloadFile: openFileMutation.mutate,
    openDownloadLocation: openLocationMutation.mutate,
    retryDownload,
    refreshDownloads,
    isLoading: query.isLoading,
    isRefreshing: query.isRefetching,
    activeCount: downloads.filter((download) => download.status === 'in_progress').length,
    queuedCount: downloads.filter((download) => download.status === 'requested').length,
  };
}
