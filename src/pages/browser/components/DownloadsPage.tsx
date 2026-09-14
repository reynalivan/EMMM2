import { DOWNLOAD_STATUS_BADGE } from '../downloadStatusBadge';
import { getDownloadProgress, getQueuePosition } from '../downloadPresentation';
import { useDownloads } from '../hooks/useDownloads';
import { Download, Globe, Inbox, RefreshCw, RotateCcw, Trash2, X } from 'lucide-react';
import type { DownloadStatus } from '../types';
import type { BrowserDownloadItem } from '../types';
import { formatBytes } from '@/shared/lib/utils/formatters';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '@/app/store';
import {
  WorkspaceContextBar,
  WorkspacePageContent,
  WorkspacePageFrame,
} from '@/shared/ui/components/layout/WorkspacePageFrame';
import { TopBarActionsPortal } from '@/widgets/top-bar';
import { DownloadRow } from './DownloadManagerPanel';
import VirtualList from '@/shared/ui/components/ui/VirtualList';
import WorkspacePanelSkeleton from '@/shared/ui/components/ui/WorkspacePanelSkeleton';

const getDownloadItemKey = (item: BrowserDownloadItem) => item.id;

export default function DownloadsPage() {
  const { t } = useTranslation('browser');
  const setWorkspaceView = useAppStore((state) => state.setWorkspaceView);
  const activeGameId = useAppStore((state) => state.activeGameId);
  const {
    downloads,
    deleteDownload,
    cancelDownload,
    pauseDownload,
    resumeDownload,
    refreshDownloadLink,
    openDownloadSource,
    retryDownload,
    refreshDownloads,
    isLoading,
    isRefreshing,
  } = useDownloads(activeGameId);
  const usesVirtualList = downloads.length > 80;

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
    <WorkspacePageFrame context={<WorkspaceContextBar description={t('downloads.description')} />}>
      <TopBarActionsPortal>
        <button
          className="btn btn-ghost btn-sm gap-2 whitespace-nowrap"
          onClick={() => setWorkspaceView('browser')}
        >
          <Globe size={16} />
          {t('tabs.discover')}
        </button>
        <button
          className="btn btn-ghost btn-sm gap-2 whitespace-nowrap"
          onClick={() => setWorkspaceView('mod-inbox')}
        >
          <Inbox size={16} />
          {t('downloads.open_mod_inbox')}
        </button>
        <button
          className="btn btn-ghost btn-sm btn-square"
          onClick={() => void refreshDownloads()}
          disabled={isRefreshing}
          title={t('downloads.refresh')}
          aria-label={t('downloads.refresh')}
        >
          <RefreshCw size={16} className={isRefreshing ? 'animate-spin' : undefined} />
        </button>
      </TopBarActionsPortal>
      <WorkspacePageContent className="space-y-4">
        {isLoading ? (
          <WorkspacePanelSkeleton variant="list" />
        ) : downloads.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-20 text-center text-base-content/70">
            <Download size={48} className="mb-4 text-base-content/30" />
            <h2 className="text-lg font-semibold text-base-content">{t('downloads.empty')}</h2>
          </div>
        ) : usesVirtualList ? (
          <div className="workspace-surface flex min-h-0 max-h-[calc(100vh-15rem)] flex-col overflow-hidden">
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
                  onDelete={(deleteFile) => deleteDownload({ id: item.id, deleteFile })}
                  onCancel={() => cancelDownload(item.id)}
                  onPause={() => pauseDownload(item.id)}
                  onResume={() => resumeDownload(item.id)}
                  onRefreshLink={() => refreshDownloadLink(item.id)}
                  onOpenSource={() => openDownloadSource(item.id)}
                  onRetry={() => retryDownload(item.id)}
                />
              )}
            />
          </div>
        ) : (
          <div className="workspace-surface overflow-x-auto">
            <table className="table tabular-nums">
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
                              className="btn btn-sm btn-outline"
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
      </WorkspacePageContent>
    </WorkspacePageFrame>
  );
}
