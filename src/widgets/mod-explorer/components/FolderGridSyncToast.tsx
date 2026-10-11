import { useEffect, useState } from 'react';
import { LoaderCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '@/app/store';
import { LiquidSurface } from '@/shared/ui/liquid';

interface FolderGridSyncToastProps {
  recoveryStatus: 'ready' | 'syncing' | 'failed';
}

export default function FolderGridSyncToast({ recoveryStatus }: FolderGridSyncToastProps) {
  const { t } = useTranslation(['grid', 'layout']);
  const activeGameId = useAppStore((state) => state.activeGameId);
  const reconcileProgress = useAppStore((state) =>
    activeGameId ? (state.diskReconcileByGame[activeGameId]?.progress ?? null) : null,
  );
  const visibleProgress =
    reconcileProgress?.reason === 'InternalMutation' ? null : reconcileProgress;
  const scanning = visibleProgress?.phase === 'ScanningRoots';
  const projecting = visibleProgress?.phase === 'Projecting';
  const totalUnits = visibleProgress?.total_units ?? null;
  const measured = (scanning || projecting) && totalUnits !== null && totalUnits > 0;
  const completedUnits = Math.min(visibleProgress?.completed_units ?? 0, totalUnits ?? 0);
  const activity = visibleProgress
    ? t(
        scanning
          ? 'game_selector.indexing.activation.scanning_without_root'
          : `game_selector.indexing.activation.phase.${visibleProgress.phase}`,
        { ns: 'layout' },
      )
    : null;
  const isBlockingRecovery = recoveryStatus === 'syncing';
  const isSyncing = isBlockingRecovery || visibleProgress !== null;
  const [isMounted, setIsMounted] = useState(isSyncing);
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    if (isSyncing) {
      setIsMounted(true);
      const showTimer = window.setTimeout(() => setIsVisible(true), 250);
      return () => window.clearTimeout(showTimer);
    }

    setIsVisible(false);
    const unmountTimer = window.setTimeout(() => setIsMounted(false), 150);
    return () => window.clearTimeout(unmountTimer);
  }, [isSyncing]);

  if (!isMounted) {
    return null;
  }

  return (
    <div
      className={`pointer-events-none absolute inset-x-4 bottom-20 z-[var(--workspace-layer-overlay)] flex justify-center transition-opacity duration-150 motion-reduce:transition-none ${
        isVisible ? 'opacity-100' : 'opacity-0'
      }`}
      data-testid="workspace-reconcile-sync-toast"
    >
      <LiquidSurface
        liquidRole="control"
        className="max-w-full rounded-xl shadow-lg"
        contentClassName="h-auto"
      >
        <div
          className="flex max-w-[min(28rem,calc(100vw-2rem))] items-center gap-3 px-3 py-2"
          role="status"
        >
          <LoaderCircle
            size={16}
            className="shrink-0 animate-spin text-info motion-reduce:animate-none"
            aria-hidden="true"
          />
          <div className="min-w-0">
            <p className="text-xs font-medium text-base-content">
              {t(isBlockingRecovery ? 'banners.disk_syncing' : 'banners.disk_refreshing')}
            </p>
            {activity && <p className="mt-0.5 text-[10px] text-base-content/65">{activity}</p>}
            {measured && (
              <div className="mt-1 flex items-center gap-2">
                <progress
                  className="progress progress-info h-1.5 w-28"
                  value={completedUnits}
                  max={totalUnits}
                  aria-label={activity ?? undefined}
                />
                <span className="shrink-0 text-[10px] tabular-nums text-base-content/65">
                  {t(
                    projecting
                      ? 'game_selector.indexing.activation.mods_complete'
                      : 'game_selector.indexing.activation.folders_complete',
                    { ns: 'layout', completed: completedUnits, total: totalUnits },
                  )}
                </span>
              </div>
            )}
            {visibleProgress?.current_root && (
              <p
                className="mt-0.5 truncate text-[10px] text-base-content/60"
                title={visibleProgress.current_root}
              >
                {visibleProgress.current_root}
              </p>
            )}
          </div>
        </div>
      </LiquidSurface>
    </div>
  );
}
