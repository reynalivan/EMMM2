import { Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import type { DiskReconcileProgress } from '@/shared/api/tauri/bindings';

interface GameIndexingOverlayProps {
  gameName: string;
  progress: DiskReconcileProgress | null;
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

export function GameIndexingOverlay({ gameName, progress }: GameIndexingOverlayProps) {
  const { t } = useTranslation('layout');
  const rootName = displayRootName(progress?.current_root);
  const totalRoots = progress?.total_units ?? null;
  const completedRoots = Math.min(progress?.completed_units ?? 0, totalRoots ?? 0);
  const hasDeterminateProgress = totalRoots !== null && totalRoots > 0;
  const progressPercent = hasDeterminateProgress
    ? Math.round((completedRoots / totalRoots) * 100)
    : null;
  const activity =
    progress?.phase === 'ScanningRoots'
      ? rootName
        ? t('game_selector.indexing.activation.scanning_root', { root: rootName })
        : t('game_selector.indexing.activation.scanning_without_root')
      : progress
        ? t(`game_selector.indexing.activation.phase.${progress.phase}`)
        : t('game_selector.indexing.activation.preparing');

  return (
    <section
      className="fixed inset-0 z-[calc(var(--workspace-layer-modal)+1)] grid place-items-center bg-base-100 p-6"
      role="status"
      aria-live="polite"
      aria-busy="true"
    >
      <div className="w-full max-w-md space-y-6 text-center">
        <div className="space-y-3">
          <Loader2
            className="mx-auto h-8 w-8 animate-spin text-primary motion-reduce:animate-none"
            aria-hidden="true"
          />
          <h1 className="text-2xl font-bold text-base-content">
            {t('game_selector.indexing.activation.title', { game: gameName })}
          </h1>
          <p className="text-sm text-base-content/70">
            {t('game_selector.indexing.activation.description')}
          </p>
        </div>

        <div className="space-y-4 rounded-2xl border border-base-content/10 bg-base-200/50 p-6 text-left shadow-sm">
          <div className="flex items-baseline justify-between gap-4">
            <span className="text-sm font-medium text-base-content/70">
              {t('game_selector.indexing.activation.progress_label')}
            </span>
            {hasDeterminateProgress && (
              <span className="text-sm font-semibold text-base-content">
                {t('game_selector.indexing.activation.folders_complete', {
                  completed: completedRoots,
                  total: totalRoots,
                })}
              </span>
            )}
          </div>

          {hasDeterminateProgress && progressPercent !== null ? (
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
            <div className="flex items-center gap-2 text-sm text-base-content/65">
              <Loader2
                className="h-4 w-4 animate-spin text-primary motion-reduce:animate-none"
                aria-hidden="true"
              />
              <span>{activity}</span>
            </div>
          )}

          {hasDeterminateProgress && (
            <p className="text-sm text-base-content/70">{activity}</p>
          )}
          {rootName && progress?.phase !== 'ScanningRoots' && (
            <p className="text-xs text-base-content/60">
              {t('game_selector.indexing.activation.current_root', { root: rootName })}
            </p>
          )}
        </div>
      </div>
    </section>
  );
}
