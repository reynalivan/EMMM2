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
