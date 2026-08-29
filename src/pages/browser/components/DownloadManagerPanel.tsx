import { DOWNLOAD_STATUS_BADGE } from '../downloadStatusBadge';
import { useShallow } from 'zustand/react/shallow';
import { useBrowserStore } from '@/app/store/useBrowserStore';
import { useAppStore } from '@/app/store/useAppStore';
import { useDownloads } from '../hooks/useDownloads';
import { useTranslation } from 'react-i18next';
import type { BrowserDownloadItem } from '../types';
import { formatBytes } from '@/shared/lib/utils/formatters';

export function DownloadManagerPanel() {
  const { t } = useTranslation(['browser']);
  const { isDownloadPanelOpen, closeDownloadPanel } = useBrowserStore(
    useShallow((state) => ({
      isDownloadPanelOpen: state.isDownloadPanelOpen,
      closeDownloadPanel: state.closeDownloadPanel,
    })),
  );

  const { downloads, deleteDownload, cancelDownload, clearImported } = useDownloads();

  return (
    <div
      id="download-manager-panel"
      className={`
        fixed top-0 right-0 h-full w-100 z-60 bg-base-200 shadow-2xl
        transition-transform duration-300 ease-in-out flex flex-col
        ${isDownloadPanelOpen ? 'translate-x-0' : 'translate-x-full'}
      `}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-base-300">
        <div className="flex items-center gap-2">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="w-5 h-5 text-primary"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
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
            ✕
          </button>
        </div>
      </div>

      {/* Toolbar */}
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

      {/* List */}
      <div className="flex-1 overflow-y-auto py-2">
        {downloads.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full gap-3 text-base-content/40">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              className="w-12 h-12"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
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
              onDelete={(deleteFile) => deleteDownload({ id: item.id, deleteFile })}
              onCancel={() => cancelDownload(item.id)}
            />
          ))
        )}
      </div>
    </div>
  );
}

interface RowProps {
  item: BrowserDownloadItem;
  onDelete: (deleteFile: boolean) => void;
  onCancel: () => void;
}

function DownloadRow({ item, onDelete, onCancel }: RowProps) {
  const { t } = useTranslation(['browser']);
  const badge = DOWNLOAD_STATUS_BADGE[item.status];
  const progress =
    item.bytes_total && item.bytes_total > 0
      ? Math.round((item.bytes_received / item.bytes_total) * 100)
      : null;

  return (
    <div
      className={`
        flex items-start gap-3 px-4 py-3 border-b border-base-300/50
        hover:bg-base-300/30 transition-colors
      `}
    >
      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium text-base-content truncate max-w-50">{item.filename}</p>
          <span className={`badge badge-sm ${badge.cls}`}>{t(badge.labelKey)}</span>
        </div>

        {/* Progress bar */}
        {item.status === 'in_progress' && progress !== null && (
          <div className="mt-1">
            <progress className="progress progress-primary w-full h-1" value={progress} max="100" />
            <p className="text-xs text-base-content/50 mt-0.5">
              {formatBytes(item.bytes_received)}
              {item.bytes_total ? ` / ${formatBytes(item.bytes_total)}` : ''}
            </p>
          </div>
        )}

        {/* Error */}
        {item.status === 'failed' && item.error_msg && (
          <p className="text-xs text-error mt-1 truncate">{item.error_msg}</p>
        )}
      </div>

      {/* Actions */}
      <div className="flex gap-1">
        {item.status === 'in_progress' && (
          <button
            className="btn btn-ghost btn-xs text-warning"
            onClick={onCancel}
            title={t('downloads.cancel_title')}
          >
            ✕
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
          >
            {t('downloads.delete')}
          </button>
        )}
      </div>
    </div>
  );
}
