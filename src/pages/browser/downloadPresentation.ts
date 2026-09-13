import type { BrowserDownloadItem } from './types';

export function getDownloadProgress(download: BrowserDownloadItem): number | null {
  if (!download.bytes_total || download.bytes_total <= 0) return null;

  return Math.min(100, Math.round((download.bytes_received / download.bytes_total) * 100));
}

/** FIFO queue position, calculated from the scheduler's admission order. */
export function getQueuePosition(downloads: BrowserDownloadItem[], id: string): number | null {
  const queuedDownloads = downloads
    .filter((download) => download.status === 'requested')
    .sort((left, right) => left.queue_order - right.queue_order);
  const index = queuedDownloads.findIndex((download) => download.id === id);

  return index === -1 ? null : index + 1;
}

/** Maps persisted backend codes and legacy request text to user-facing copy. */
export function getDownloadFailureMessageKey(errorMessage: string | null): string {
  switch (errorMessage) {
    case 'download.timeout':
      return 'downloads.failure.timeout';
    case 'download.offline':
      return 'downloads.failure.offline';
    case 'download.access_denied':
      return 'downloads.failure.access_denied';
    case 'download.not_found':
      return 'downloads.failure.not_found';
    case 'download.server':
      return 'downloads.failure.server';
    default:
      if (errorMessage?.toLowerCase().includes('timed out')) return 'downloads.failure.timeout';
      if (errorMessage?.toLowerCase().includes('connect')) return 'downloads.failure.offline';
      return 'downloads.failure.unknown';
  }
}
