import { DOWNLOAD_STATUS_BADGE } from '../downloadStatusBadge';
import { getDownloadProgress, getQueuePosition } from '../downloadPresentation';
import { useDownloads } from '../hooks/useDownloads';
import { Download, RefreshCw, RotateCcw, Trash2, X } from 'lucide-react';
import type { DownloadStatus } from '../types';
import type { BrowserDownloadItem } from '../types';
import { formatBytes } from '@/shared/lib/utils/formatters';
import { useTranslation } from 'react-i18next';

export default function DownloadsPage() {
  const { t } = useTranslation('browser');
  const {
    downloads,
    deleteDownload,
    cancelDownload,
    retryDownload,
    refreshDownloads,
    isRefreshing,
  } = useDownloads();

  const renderStatus = (status: DownloadStatus) => {
    const badge = DOWNLOAD_STATUS_BADGE[status] ?? DOWNLOAD_STATUS_BADGE.requested;
    const Icon = badge.icon;
    return (
      <span className={`badge ${badge.cls} gap-1`}>
        {Icon && <Icon size={12} className={badge.spin ? 'animate-spin' : undefined} />}
        {t(badge.labelKey)}
      </span>
    );
  };

  return (
    <div className="flex flex-col h-full bg-base-100 overflow-hidden relative">
      <div className="w-full bg-base-200 border-b border-base-300 p-6 z-10">
        <div className="flex items-center justify-between max-w-7xl mx-auto gap-4">
          <div>
            <h1 className="text-3xl font-bold flex items-center gap-3">
              <Download size={32} className="text-primary" />
              {t('downloads.title')}
            </h1>
            <p className="text-base-content/75 mt-1">{t('welcome.description')}</p>
          </div>
          <div className="flex items-center gap-2">
            <button
              className="btn btn-outline btn-sm btn-square"
              onClick={() => void refreshDownloads()}
              disabled={isRefreshing}
              title={t('downloads.refresh')}
              aria-label={t('downloads.refresh')}
            >
              <RefreshCw size={16} className={isRefreshing ? 'animate-spin' : undefined} />
            </button>
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto w-full p-6">
        <div className="max-w-7xl mx-auto space-y-4">
          {downloads.length === 0 ? (
            <div className="flex flex-col items-center justify-center p-20 text-center opacity-70">
              <Download size={64} className="mb-4" />
              <h2 className="text-xl font-semibold">{t('downloads.empty')}</h2>
              <p>{t('welcome.description')}</p>
            </div>
          ) : (
            <div className="overflow-x-auto bg-base-200/50 rounded-2xl border border-base-300">
              <table className="table">
                <thead>
                  <tr>
                    <th>{t('downloads.table_filename')}</th>
                    <th>{t('downloads.table_status')}</th>
                    <th>{t('downloads.table_progress')}</th>
                    <th className="text-right">{t('downloads.table_actions')}</th>
                  </tr>
                </thead>
                <tbody>
                  {downloads.map((item: BrowserDownloadItem) => {
                    const progress = getDownloadProgress(item);
                    const queuePosition = getQueuePosition(downloads, item.id);

                    return (
                      <tr key={item.id} className="hover">
                        <td className="w-1/3">
                          <p
                            className="font-semibold text-base-content truncate max-w-75"
                            title={item.filename}
                          >
                            {item.filename}
                          </p>
                        </td>
                        <td className="w-1/6">
                          {renderStatus(item.status)}
                          {item.status === 'failed' && (
                            <p className="text-xs text-error mt-1 max-w-37.5">
                              {t('downloads.failure_details')}
                            </p>
                          )}
                        </td>
                        <td className="w-1/4">
                          {item.status === 'requested' && queuePosition !== null ? (
                            <span className="text-sm text-base-content/75">
                              {t('downloads.queue_position', { position: queuePosition })}
                            </span>
                          ) : item.status === 'in_progress' ? (
                            <div>
                              <progress
                                className="progress progress-primary w-full"
                                {...(progress === null ? {} : { value: progress, max: 100 })}
                              />
                              <div className="flex justify-between text-xs mt-1 text-base-content/75">
                                <span>{formatBytes(item.bytes_received)}</span>
                                <span>
                                  {item.bytes_total
                                    ? formatBytes(item.bytes_total)
                                    : t('downloads.unknown_size')}
                                </span>
                              </div>
                            </div>
                          ) : (
                            <span className="text-sm text-base-content/75">
                              {item.bytes_total
                                ? formatBytes(item.bytes_total)
                                : formatBytes(item.bytes_received)}
                            </span>
                          )}
                        </td>
                        <td className="w-1/4 text-right">
                          <div className="flex items-center justify-end gap-2">
                            {(item.status === 'requested' || item.status === 'in_progress') && (
                              <button
                                className="btn btn-sm btn-warning"
                                onClick={() => cancelDownload(item.id)}
                              >
                                <X size={16} /> {t('common:action.cancel')}
                              </button>
                            )}
                            {(item.status === 'failed' || item.status === 'canceled') && (
                              <button
                                className="btn btn-sm btn-primary btn-outline gap-1"
                                onClick={() => retryDownload(item.id)}
                              >
                                <RotateCcw size={16} /> {t('downloads.retry')}
                              </button>
                            )}
                            {(item.status === 'finished' ||
                              item.status === 'failed' ||
                              item.status === 'canceled' ||
                              item.status === 'imported') && (
                              <button
                                className="btn btn-sm btn-ghost text-error"
                                onClick={() => deleteDownload({ id: item.id, deleteFile: false })}
                                title={t('downloads.delete_title')}
                                aria-label={t('downloads.delete_title')}
                              >
                                <Trash2 size={16} />
                              </button>
                            )}
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
