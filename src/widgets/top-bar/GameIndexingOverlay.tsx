import { AlertCircle, Loader2, RotateCcw } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type {
  DiskReconcileProgress,
  GameActivationPhase,
  OnboardingIndexingBackgroundGameStatus,
  OnboardingIndexingSnapshotProgress,
} from '@/shared/api/tauri/bindings';

interface GameIndexingOverlayProps {
  gameName: string;
  progress: DiskReconcileProgress | null;
  snapshotProgress?: OnboardingIndexingSnapshotProgress;
  backgroundPhase?: OnboardingIndexingBackgroundGameStatus['phase'];
  phase?: GameActivationPhase;
  error?: string | null;
  onRetry?: () => void;
}

function displayRootName(root: string | null | undefined): string | null {
  const normalized = root?.trim().replace(/[\\/]+$/, '');
  if (!normalized || normalized === '.') {
    return null;
  }
  const separatorIndex = Math.max(normalized.lastIndexOf('/'), normalized.lastIndexOf('\\'));
  const name = normalized.slice(separatorIndex + 1);
  return name.replace(/^#+/, '') || null;
}

export function GameIndexingOverlay({
  gameName,
  progress,
  snapshotProgress,
  backgroundPhase,
  phase = 'syncing',
  error,
  onRetry,
}: GameIndexingOverlayProps) {
  const { t } = useTranslation('layout');
  const failed = phase === 'failed' || phase === 'source_unavailable';
  const rootName = displayRootName(progress?.current_root ?? snapshotProgress?.current_root);
  const totalRoots = progress?.total_units ?? snapshotProgress?.total_roots ?? null;
  const completedRoots = Math.min(
    progress?.completed_units ?? snapshotProgress?.completed_roots ?? 0,
    totalRoots ?? 0,
  );
  const hasDeterminateProgress = totalRoots !== null && totalRoots > 0;
  const scanning = progress
    ? progress.phase === 'ScanningRoots'
    : snapshotProgress?.phase === 'Metadata' || snapshotProgress?.phase === 'Classifying';
  const progressPercent = hasDeterminateProgress
    ? Math.round((completedRoots / totalRoots) * 100)
    : null;
  const activity = progress
    ? progress.phase === 'ScanningRoots'
      ? rootName
        ? t('game_selector.indexing.activation.scanning_root', { root: rootName })
        : t('game_selector.indexing.activation.scanning_without_root')
      : t(`game_selector.indexing.activation.phase.${progress.phase}`)
    : backgroundPhase === 'Prepared' || backgroundPhase === 'Applying'
      ? t(`game_selector.indexing.phase.${backgroundPhase}`)
      : snapshotProgress
        ? t(`game_selector.indexing.activation.snapshot_phase.${snapshotProgress.phase}`)
        : backgroundPhase
          ? t(`game_selector.indexing.phase.${backgroundPhase}`)
          : t('game_selector.indexing.activation.preparing');

  return (
    <section
      className="flex h-full items-center justify-center overflow-y-auto px-4 pt-[var(--workspace-topbar-height)] pb-6"
      role={failed ? 'alert' : 'status'}
      aria-live="polite"
      aria-busy={!failed}
    >
      <div className="w-full max-w-lg space-y-5 rounded-xl border border-base-content/10 bg-base-100/95 p-6 text-left shadow-lg backdrop-blur">
        <div className="flex items-start gap-3">
          {failed ? (
            <AlertCircle className="mt-0.5 h-5 w-5 shrink-0 text-warning" aria-hidden="true" />
          ) : (
            <Loader2
              className="mt-0.5 h-5 w-5 shrink-0 animate-spin text-primary motion-reduce:animate-none"
              aria-hidden="true"
            />
          )}
          <div className="min-w-0">
            <h1 className="text-sm font-bold text-base-content">
              {t(
                failed
                  ? 'game_selector.indexing.activation.failed_title'
                  : 'game_selector.indexing.activation.title',
                { game: gameName },
              )}
            </h1>
            <p className="text-xs text-base-content/70">
              {t(
                phase === 'source_unavailable'
                  ? 'game_selector.indexing.activation.source_unavailable_description'
                  : failed
                    ? 'game_selector.indexing.activation.failed_description'
                    : 'game_selector.indexing.activation.description',
              )}
            </p>
          </div>
        </div>

        {failed ? (
          <div className="space-y-4">
            {error && <p className="text-sm text-base-content/70">{error}</p>}
            {onRetry && (
              <button type="button" className="btn btn-primary btn-sm" onClick={onRetry}>
                <RotateCcw size={14} aria-hidden="true" />
                {t('game_selector.retry')}
              </button>
            )}
          </div>
        ) : (
          <div className="space-y-2">
            <div className="flex items-baseline justify-between gap-4">
              <span className="text-xs font-medium text-base-content/70">
                {t('game_selector.indexing.activation.progress_label')}
              </span>
              {hasDeterminateProgress && (
                <span className="text-xs font-semibold text-base-content">
                  {t('game_selector.indexing.activation.folders_complete', {
                    completed: completedRoots,
                    total: totalRoots,
                  })}
                </span>
              )}
            </div>

            {hasDeterminateProgress && progressPercent !== null && scanning ? (
              <div
                className="h-2 overflow-hidden rounded-full bg-base-300/80"
                role="progressbar"
                aria-label={t('game_selector.indexing.activation.progress_label')}
                aria-valuemin={0}
                aria-valuemax={totalRoots ?? undefined}
                aria-valuenow={completedRoots}
              >
                <div
                  className="h-full bg-primary transition-[width] duration-150 ease-out motion-reduce:transition-none"
                  style={{ width: `${progressPercent}%` }}
                />
              </div>
            ) : (
              <div className="flex items-center gap-2 text-xs text-base-content/65">
                <Loader2
                  className="h-4 w-4 animate-spin text-primary motion-reduce:animate-none"
                  aria-hidden="true"
                />
                <span>{activity}</span>
              </div>
            )}

            {hasDeterminateProgress && scanning && (
              <p className="text-xs text-base-content/70">{activity}</p>
            )}
            {snapshotProgress && snapshotProgress.folders_classified > 0 && (
              <p className="text-xs text-base-content/60">
                {t('game_selector.indexing.activation.folders_classified', {
                  count: snapshotProgress.folders_classified,
                })}
              </p>
            )}
            {rootName &&
              !scanning &&
              (Boolean(progress?.current_root) || snapshotProgress?.phase === 'Rechecking') && (
                <p className="text-xs text-base-content/60">
                  {t('game_selector.indexing.activation.current_root', { root: rootName })}
                </p>
              )}
          </div>
        )}
      </div>
    </section>
  );
}
