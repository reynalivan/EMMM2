import { ExternalLink, Pause, Play, RefreshCw, RotateCcw, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { BrowserDownloadItem } from '../types';
import { DownloadItemActions } from './DownloadItemActions';

export interface DownloadTransferActionsProps {
  item: BrowserDownloadItem;
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

export function DownloadTransferActions(props: DownloadTransferActionsProps) {
  const { t } = useTranslation('browser');
  const { item } = props;
  const controls = [
    {
      visible:
        ['requested', 'in_progress', 'paused'].includes(item.status) ||
        (item.status === 'failed' && Boolean(item.can_resume)),
      key: 'cancel_title',
      Icon: X,
      onClick: props.onCancel,
    },
    {
      visible: item.status === 'in_progress' && Boolean(item.can_resume),
      key: 'pause_title',
      Icon: Pause,
      onClick: props.onPause,
    },
    {
      visible: ['paused', 'failed'].includes(item.status) && Boolean(item.can_resume),
      key: 'resume_title',
      Icon: Play,
      onClick: props.onResume,
    },
    {
      visible: item.status === 'failed' && !item.can_resume && Boolean(item.tab_label),
      key: 'refresh_link_title',
      Icon: RefreshCw,
      onClick: props.onRefreshLink,
    },
    {
      visible: item.status === 'failed' && !item.can_resume && Boolean(item.source_url),
      key: 'open_source_title',
      Icon: ExternalLink,
      onClick: props.onOpenSource,
    },
    {
      visible: ['failed', 'canceled'].includes(item.status),
      key: 'retry_title',
      Icon: RotateCcw,
      onClick: props.onRetry,
    },
  ];
  return (
    <div className="flex items-center justify-end gap-1">
      {controls
        .filter((control) => control.visible)
        .map(({ key, Icon, onClick }) => (
          <button
            key={key}
            className="btn btn-ghost btn-xs"
            onClick={onClick}
            title={t(`downloads.${key}`)}
            aria-label={t(`downloads.${key}`)}
          >
            <Icon size={14} />
          </button>
        ))}
      <DownloadItemActions {...props} />
    </div>
  );
}
