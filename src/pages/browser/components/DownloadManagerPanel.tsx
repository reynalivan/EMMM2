import { DOWNLOAD_STATUS_BADGE } from '../downloadStatusBadge';
import {
  getDownloadFailureMessageKey,
  getDownloadProgress,
  getQueuePosition,
} from '../downloadPresentation';
import { useShallow } from 'zustand/react/shallow';
import { useBrowserStore } from '@/entities/browser';
import { useAppStore } from '@/app/store';
import { useDownloads } from '../hooks/useDownloads';
import { useTranslation } from 'react-i18next';
import type { BrowserDownloadItem } from '../types';
import { formatBytes } from '@/shared/lib/utils/formatters';
import { RefreshCw, X } from 'lucide-react';
import VirtualList from '@/shared/ui/components/ui/VirtualList';
import WorkspacePanelSkeleton from '@/shared/ui/components/ui/WorkspacePanelSkeleton';
import type { BrowserSidePanelLayout } from '../browserSurfacePresentation';
import { DownloadTransferActions } from './DownloadTransferActions';

const getDownloadItemKey = (item: BrowserDownloadItem) => item.id;

interface DownloadManagerPanelProps {
  layout: BrowserSidePanelLayout;
}

export function DownloadManagerPanel({ layout }: DownloadManagerPanelProps) {
  const { t } = useTranslation(['browser']);
  const activeGameId = useAppStore((state) => state.activeGameId);
  const { closeDownloadPanel } = useBrowserStore(
    useShallow((state) => ({
      closeDownloadPanel: state.closeDownloadPanel,
    })),
  );

  const {
    downloads,
    deleteDownload,
    cancelDownload,
    pauseDownload,
    resumeDownload,
    refreshDownloadLink,
    openDownloadSource,
    renameDownload,
    openDownloadFile,
    openDownloadLocation,
    retryDownload,
    refreshDownloads,
    isLoading,
    isRefreshing,
  } = useDownloads(activeGameId, { subscribe: false });
  const usesVirtualList = downloads.length > 80;

  return (
    <aside
      id="download-manager-panel"
      aria-label={t('downloads.title')}
      className={`
        flex h-full min-h-0 flex-col border-l border-base-300 bg-base-200 shadow-2xl
        ${layout === 'docked' ? 'w-[400px] shrink-0' : 'w-full'}
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

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden py-2">
        {isLoading ? (
          <WorkspacePanelSkeleton variant="list" />
        ) : downloads.length === 0 ? (
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
        ) : usesVirtualList ? (
          <VirtualList
            items={downloads}
            getItemKey={getDownloadItemKey}
            estimateSize={() => 92}
            ariaLabel={t('downloads.title')}
            className="scrollbar-thin scrollbar-track-transparent scrollbar-thumb-base-content/20"
            renderItem={(item) => (
              <DownloadRow
                item={item}
                queuePosition={getQueuePosition(downloads, item.id)}
                onRemoveFromList={() => deleteDownload({ id: item.id, deleteFile: false })}
                onDeleteFile={() => deleteDownload({ id: item.id, deleteFile: true })}
                onRename={(filename) => renameDownload({ id: item.id, filename })}
                onOpenFile={() => openDownloadFile(item.id)}
                onOpenLocation={() => openDownloadLocation(item.id)}
                onCancel={() => cancelDownload(item.id)}
                onPause={() => pauseDownload(item.id)}
                onResume={() => resumeDownload(item.id)}
                onRefreshLink={() => refreshDownloadLink(item.id)}
                onOpenSource={() => openDownloadSource(item.id)}
                onRetry={() => retryDownload(item.id)}
              />
            )}
          />
        ) : (
          <div className="overflow-y-auto">
            {downloads.map((item) => (
              <DownloadRow
                key={item.id}
                item={item}
                queuePosition={getQueuePosition(downloads, item.id)}
                onRemoveFromList={() => deleteDownload({ id: item.id, deleteFile: false })}
                onDeleteFile={() => deleteDownload({ id: item.id, deleteFile: true })}
                onRename={(filename) => renameDownload({ id: item.id, filename })}
                onOpenFile={() => openDownloadFile(item.id)}
                onOpenLocation={() => openDownloadLocation(item.id)}
                onCancel={() => cancelDownload(item.id)}
                onPause={() => pauseDownload(item.id)}
                onResume={() => resumeDownload(item.id)}
                onRefreshLink={() => refreshDownloadLink(item.id)}
                onOpenSource={() => openDownloadSource(item.id)}
                onRetry={() => retryDownload(item.id)}
              />
            ))}
          </div>
        )}
      </div>
    </aside>
  );
}

interface RowProps {
  item: BrowserDownloadItem;
  queuePosition: number | null;
  onRemoveFromList: () => void;
  onDeleteFile: () => void;
  onRename: (filename: string) => Promise<unknown>;
  onOpenFile: () => void;
  onOpenLocation: () => void;
  onCancel: () => void;
  onPause: () => void;
  onResume: () => void;
  onRefreshLink: () => void;
  onOpenSource: () => void;
  onRetry: () => void;
}

export function DownloadRow({
  item,
  queuePosition,
  onRemoveFromList,
  onDeleteFile,
  onRename,
  onOpenFile,
  onOpenLocation,
  onCancel,
  onPause,
  onResume,
  onRefreshLink,
  onOpenSource,
  onRetry,
}: RowProps) {
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

        {item.status === 'paused' && (
          <p className="mt-1 text-xs text-base-content/55">{t('downloads.status.paused')}</p>
        )}

        {item.status === 'failed' && (
          <p className="mt-1 text-xs text-error">
            {t(getDownloadFailureMessageKey(item.error_msg))}
          </p>
        )}
      </div>

      <DownloadTransferActions
        item={item}
        onRemoveFromList={onRemoveFromList}
        onDeleteFile={onDeleteFile}
        onRename={onRename}
        onOpenFile={onOpenFile}
        onOpenLocation={onOpenLocation}
        onCancel={onCancel}
        onPause={onPause}
        onResume={onResume}
        onRefreshLink={onRefreshLink}
        onOpenSource={onOpenSource}
        onRetry={onRetry}
      />
    </div>
  );
}
