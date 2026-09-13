import { AlertTriangle, LoaderCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { StorageSizeBackfillStatus } from '../hooks/useStorageSizeBackfill';

interface DashboardStorageBackfillStatusProps {
  status: StorageSizeBackfillStatus | null;
  onRetry: () => void;
}

export function DashboardStorageBackfillStatus({
  status,
  onRetry,
}: DashboardStorageBackfillStatusProps) {
  const { t } = useTranslation(['dashboard']);

  if (!status || status.state === 'Idle' || status.state === 'Completed') {
    return null;
  }

  if (status.state === 'Failed') {
    return (
      <div role="alert" className="alert alert-error alert-soft" aria-live="polite">
        <AlertTriangle size={20} />
        <div>
          <h2 className="font-bold">{t('storage_backfill.failed_title')}</h2>
          <p className="text-sm">{status.errors[0] ?? t('storage_backfill.failed_message')}</p>
        </div>
        <button type="button" className="btn btn-sm btn-outline" onClick={onRetry}>
          {t('storage_backfill.retry')}
        </button>
      </div>
    );
  }

  return (
    <div role="status" className="alert alert-info alert-soft" aria-live="polite">
      <LoaderCircle size={20} className="animate-spin" />
      <div>
        <h2 className="font-bold">{t('storage_backfill.running_title')}</h2>
        <p className="text-sm">
          {t('storage_backfill.running_progress', {
            completed: status.completed_games,
            total: status.total_games,
          })}
        </p>
      </div>
    </div>
  );
}
