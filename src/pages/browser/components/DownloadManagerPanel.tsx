import { DOWNLOAD_STATUS_BADGE } from '../downloadStatusBadge';
import { getDownloadProgress, getQueuePosition } from '../downloadPresentation';
import { useShallow } from 'zustand/react/shallow';
import { useBrowserStore } from '@/entities/browser';
import { useAppStore } from '@/app/store';
import { useDownloads } from '../hooks/useDownloads';
import { useTranslation } from 'react-i18next';
import type { BrowserDownloadItem } from '../types';
import { formatBytes } from '@/shared/lib/utils/formatters';
import { RefreshCw, RotateCcw, X } from 'lucide-react';

export function DownloadManagerPanel() {
  const { t } = useTranslation(['browser']);
  const { isDownloadPanelOpen, closeDownloadPanel } = useBrowserStore(
    useShallow((state) => ({
      isDownloadPanelOpen: state.isDownloadPanelOpen,
      closeDownloadPanel: state.closeDownloadPanel,
    })),
  );

  const {
    downloads,
    deleteDownload,
    cancelDownload,
    clearImported,
    retryDownload,
    refreshDownloads,
    isRefreshing,
  } = useDownloads();

  return (
    <div
      id="download-manager-panel"
      className={`
        fixed top-0 right-0 h-full w-100 z-60 bg-base-200 shadow-2xl
        transition-transform duration-300 ease-in-out flex flex-col
        ${isDownloadPanelOpen ? 'translate-x-0' : 'translate-x-full'}
      `}
    >
      <div className="flex items-center justify-between px-4 py-3 border-b border-base-300">
        <div className="flex items-center gap-2">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="w-5 h-5 text-primary"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
            />
          </svg>
          <h2 className="font-semibold text-base-content">
            {t('downloads.title')}
            {downloads.length > 0 && (
              <span className="ml-2 badge badge-primary badge-sm">{downloads.length}</span>
            )}
          </h2>
        </div>
        <div className="flex items-center gap-1">
          <button
            className="btn btn-ghost btn-xs btn-square"
            onClick={() => void refreshDownloads()}
            disabled={isRefreshing}
            title={t('downloads.refresh')}
            aria-label={t('downloads.refresh')}
          >
            <RefreshCw size={14} className={isRefreshing ? 'animate-spin' : undefined} />
          </button>
          <button
            className="btn btn-ghost btn-xs text-primary"
            onClick={() => {
              useAppStore.getState().setWorkspaceView('downloads');
              closeDownloadPanel();
            }}
            title={t('downloads.view_detail')}
          >
            {t('downloads.view_detail')}
          </button>
          <button
            id="download-panel-close-btn"
            className="btn btn-ghost btn-sm btn-circle"
            onClick={closeDownloadPanel}
            aria-label={t('downloads.close')}
          >
            <X size={16} />
          </button>
        </div>
      </div>

      {downloads.length > 0 && (
        <div className="flex items-center justify-end px-4 py-2 border-b border-base-300">
          <button
            id="download-clear-imported-btn"
            className="btn btn-ghost btn-xs"
            onClick={() => clearImported()}
          >
            {t('downloads.clear_imported')}
          </button>
        </div>
      )}

      <div className="flex-1 overflow-y-auto py-2">
        {downloads.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full gap-3 text-base-content/40">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="w-12 h-12"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1}
                d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"
              />
            </svg>
            <p className="text-sm">{t('downloads.empty')}</p>
          </div>
        ) : (
          downloads.map((item) => (
            <DownloadRow
              key={item.id}
              item={item}
              queuePosition={getQueuePosition(downloads, item.id)}
              onDelete={(deleteFile) => deleteDownload({ id: item.id, deleteFile })}
              onCancel={() => cancelDownload(item.id)}
              onRetry={() => retryDownload(item.id)}
            />
          ))
        )}
      </div>
    </div>
  );
}

interface RowProps {
  item: BrowserDownloadItem;
  queuePosition: number | null;
  onDelete: (deleteFile: boolean) => void;
  onCancel: () => void;
  onRetry: () => void;
}

function DownloadRow({ item, queuePosition, onDelete, onCancel, onRetry }: RowProps) {
  const { t } = useTranslation(['browser']);
  const badge = DOWNLOAD_STATUS_BADGE[item.status];
  const progress = getDownloadProgress(item);

  return (
    <div className="flex items-start gap-3 px-4 py-3 border-b border-base-300/50 hover:bg-base-300/30 transition-colors">
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium text-base-content truncate max-w-50">{item.filename}</p>
          <span className={`badge badge-sm ${badge.cls}`}>{t(badge.labelKey)}</span>
        </div>

        {item.status === 'requested' && queuePosition !== null && (
          <p className="text-xs text-base-content/50 mt-1">
            {t('downloads.queue_position', { position: queuePosition })}
          </p>
        )}

        {item.status === 'in_progress' && (
          <div className="mt-1">
            <progress
              className="progress progress-primary w-full h-1"
              {...(progress === null ? {} : { value: progress, max: 100 })}
            />
            <p className="text-xs text-base-content/50 mt-0.5">
              {t('downloads.progress_received', {
                received: formatBytes(item.bytes_received),
                total: item.bytes_total
                  ? formatBytes(item.bytes_total)
                  : t('downloads.unknown_size'),
              })}
            </p>
          </div>
        )}

        {item.status === 'failed' && (
          <p className="text-xs text-error mt-1">{t('downloads.failure_details')}</p>
        )}
      </div>

      <div className="flex gap-1">
        {(item.status === 'requested' || item.status === 'in_progress') && (
          <button
            className="btn btn-ghost btn-xs text-warning"
            onClick={onCancel}
            title={t('downloads.cancel_title')}
            aria-label={t('downloads.cancel_title')}
          >
            <X size={14} />
          </button>
        )}
        {(item.status === 'failed' || item.status === 'canceled') && (
          <button
            className="btn btn-ghost btn-xs text-primary"
            onClick={onRetry}
            title={t('downloads.retry_title')}
            aria-label={t('downloads.retry_title')}
          >
            <RotateCcw size={14} />
          </button>
        )}
        {(item.status === 'finished' ||
          item.status === 'failed' ||
          item.status === 'canceled' ||
          item.status === 'imported') && (
          <button
            className="btn btn-ghost btn-xs text-error"
            onClick={() => onDelete(false)}
            title={t('downloads.delete_title')}
            aria-label={t('downloads.delete_title')}
          >
            {t('downloads.delete')}
          </button>
        )}
      </div>
    </div>
  );
}
