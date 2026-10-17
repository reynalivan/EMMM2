import { useTranslation } from 'react-i18next';
import type { ApplyPreviewDiff } from '../applyPreviewDiff';
interface Props {
  diff: ApplyPreviewDiff;
  previewMode: 'full' | 'changes';
  setPreviewMode: (mode: 'full' | 'changes') => void;
  safeModeEnabled: boolean;
}
export function ApplyPreviewSummary({ diff, previewMode, setPreviewMode, safeModeEnabled }: Props) {
  const { t } = useTranslation('collections');
  return (
    <div className="shrink-0 border-b border-base-content/8 bg-base-300/25 px-5 py-2.5">
      <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
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
        <div className="flex flex-wrap items-center gap-3 text-xs text-base-content/65">
          <span>
            {t('collections:apply.diff.disable_count')}:{' '}
            <strong className="text-base-content/85 tabular-nums">{diff.disableCount}</strong>
          </span>
          <span>
            {t('collections:apply.diff.enable_count')}:{' '}
            <strong className="text-base-content/85 tabular-nums">{diff.enableCount}</strong>
          </span>
          <span>
            {t('collections:apply.diff.unchanged_count')}:{' '}
            <strong className="text-base-content/85 tabular-nums">{diff.unchangedCount}</strong>
          </span>
        </div>
        {safeModeEnabled && (
          <span className="badge badge-sm border-warning/20 bg-warning/10 text-warning/85">
            {t('collections:apply.diff.safe_mode_on')}
          </span>
        )}
      </div>

      {(diff.objectEnableCount > 0 || diff.objectDisableCount > 0) && (
        <p className="mt-2 text-xs text-base-content/70">
          {t('collections:apply.diff.object_changes', {
            enabled: diff.objectEnableCount,
            disabled: diff.objectDisableCount,
          })}
        </p>
      )}

      {diff.excludedBySafeModeCount > 0 && (
        <div className="mt-2 rounded-lg border border-warning/20 bg-warning/8 px-3 py-2 text-xs text-warning/90">
          {t('collections:apply.diff.safe_mode_exclusions', {
            count: diff.excludedBySafeModeCount,
          })}
        </div>
      )}
    </div>
  );
}
