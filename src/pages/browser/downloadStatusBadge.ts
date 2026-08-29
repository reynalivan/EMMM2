import { RefreshCw, CheckCircle2, AlertCircle } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import type { DownloadStatus } from './types';

interface DownloadStatusBadge {
  labelKey: string;
  cls: string;
  /** Only the statuses that carry one; the compact panel renders labels alone. */
  icon?: LucideIcon;
  spin?: boolean;
}

/** Single source for how each download status is labelled and coloured. */
export const DOWNLOAD_STATUS_BADGE: Record<DownloadStatus, DownloadStatusBadge> = {
  requested: { labelKey: 'downloads.status.queued', cls: 'badge-neutral' },
  in_progress: {
    labelKey: 'downloads.status.downloading',
    cls: 'badge-info',
    icon: RefreshCw,
    spin: true,
  },
  finished: { labelKey: 'downloads.status.ready', cls: 'badge-success', icon: CheckCircle2 },
  failed: { labelKey: 'downloads.status.failed', cls: 'badge-error', icon: AlertCircle },
  canceled: { labelKey: 'downloads.status.canceled', cls: 'badge-warning' },
  imported: { labelKey: 'downloads.status.imported', cls: 'badge-ghost' },
};
