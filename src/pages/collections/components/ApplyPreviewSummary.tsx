import { useTranslation } from 'react-i18next';
import type { ApplyPreviewDiff } from '../applyPreviewDiff';
import { SummaryStat } from './ApplyCollectionStatePanel';
interface Props {
  diff: ApplyPreviewDiff;
  previewMode: 'full' | 'changes';
  setPreviewMode: (mode: 'full' | 'changes') => void;
  safeModeEnabled: boolean;
}
export function ApplyPreviewSummary({ diff, previewMode, setPreviewMode, safeModeEnabled }: Props) {
  const { t } = useTranslation('collections');
  return (
    <div className="shrink-0 border-b border-base-content/5 bg-base-300/35 px-6 py-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="join" role="tablist" aria-label={t('collections:apply.diff.view_label')}>
          <button
            type="button"
            role="tab"
            aria-selected={previewMode === 'full'}
            className={`btn btn-xs join-item ${
              previewMode === 'full' ? 'btn-primary' : 'btn-ghost'
            }`}
            onClick={() => setPreviewMode('full')}
          >
            {t('collections:apply.diff.full_collection')}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={previewMode === 'changes'}
            className={`btn btn-xs join-item ${
              previewMode === 'changes' ? 'btn-primary' : 'btn-ghost'
            }`}
            onClick={() => setPreviewMode('changes')}
          >
            {t('collections:apply.diff.changes_only')}
          </button>
        </div>
        {safeModeEnabled && (
          <span className="badge badge-sm border-warning/20 bg-warning/10 text-warning/85">
            {t('collections:apply.diff.safe_mode_on')}
          </span>
        )}
      </div>

      <div className="mt-3 grid grid-cols-3 gap-2">
        <SummaryStat label={t('collections:apply.diff.disable_count')} value={diff.disableCount} />
        <SummaryStat label={t('collections:apply.diff.enable_count')} value={diff.enableCount} />
        <SummaryStat
          label={t('collections:apply.diff.unchanged_count')}
          value={diff.unchangedCount}
        />
      </div>

      {(diff.objectEnableCount > 0 || diff.objectDisableCount > 0) && (
        <p className="mt-3 text-xs text-base-content/70">
          {t('collections:apply.diff.object_changes', {
            enabled: diff.objectEnableCount,
            disabled: diff.objectDisableCount,
          })}
        </p>
      )}

      {diff.excludedBySafeModeCount > 0 && (
        <div className="mt-3 rounded-lg border border-warning/20 bg-warning/8 px-3 py-2 text-xs text-warning/90">
          {t('collections:apply.diff.safe_mode_exclusions', {
            count: diff.excludedBySafeModeCount,
          })}
        </div>
      )}
    </div>
  );
}
