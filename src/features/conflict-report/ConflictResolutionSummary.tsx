import { AlertTriangle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { ConflictResolutionSummary as ResolutionSummary } from './conflictResolution';

interface ConflictResolutionSummaryProps {
  summary: ResolutionSummary;
  isPending: boolean;
  onBack: () => void;
  onConfirm: () => void;
}

function pathName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

export default function ConflictResolutionSummary({
  summary,
  isPending,
  onBack,
  onConfirm,
}: ConflictResolutionSummaryProps) {
  const { t } = useTranslation(['scanner', 'common']);

  return (
    <section className="space-y-4">
      <div>
        <h4 className="font-semibold text-base">{t('scanner:conflict_modal.review_title')}</h4>
        <p className="text-sm text-base-content/60 mt-1">
          {t('scanner:conflict_modal.review_description')}
        </p>
      </div>

      <div className="stats stats-vertical sm:stats-horizontal bg-base-200 w-full shadow-sm">
        <div className="stat py-3">
          <div className="stat-title text-xs">{t('scanner:conflict_modal.mods_to_disable')}</div>
          <div className="stat-value text-xl">{summary.disablePaths.length}</div>
        </div>
        <div className="stat py-3">
          <div className="stat-title text-xs">{t('scanner:conflict_modal.resolved_groups')}</div>
          <div className="stat-value text-xl text-success">{summary.resolvedCount}</div>
        </div>
        <div className="stat py-3">
          <div className="stat-title text-xs">{t('scanner:conflict_modal.unresolved_groups')}</div>
          <div className="stat-value text-xl text-warning">{summary.unresolvedCount}</div>
        </div>
      </div>

      <ul className="rounded-lg border border-base-content/10 divide-y divide-base-content/10">
        {summary.disablePaths.map((path) => (
          <li key={path} className="px-3 py-2 min-w-0">
            <div className="text-sm font-medium">{pathName(path)}</div>
            <code className="block text-xs text-base-content/60 truncate" title={path}>
              {path}
            </code>
          </li>
        ))}
      </ul>

      {summary.unresolvedCount > 0 && (
        <div className="alert alert-warning text-sm">
          <AlertTriangle size={16} />
          <span>
            {t('scanner:conflict_modal.unresolved_warning', {
              count: summary.unresolvedCount,
            })}
          </span>
        </div>
      )}

      <div className="modal-action border-t border-base-content/10 pt-4">
        <button className="btn btn-sm btn-ghost" onClick={onBack} disabled={isPending}>
          {t('common:actions.back')}
        </button>
        <button
          className="btn btn-sm btn-error"
          onClick={onConfirm}
          disabled={isPending || summary.disablePaths.length === 0}
          aria-label={t('scanner:conflict_modal.disable_confirm', {
            count: summary.disablePaths.length,
          })}
        >
          {isPending ? (
            <span className="loading loading-spinner loading-xs" />
          ) : (
            t('scanner:conflict_modal.disable_confirm', {
              count: summary.disablePaths.length,
            })
          )}
        </button>
      </div>
    </section>
  );
}
