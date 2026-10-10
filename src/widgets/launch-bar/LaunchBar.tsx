import { formatAppError } from '../../shared/lib/appError';
import { AlertTriangle, Play, Shuffle } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { createPortal } from 'react-dom';
import { launchConfiguredGame, useActiveGame } from '@/entities/game';
import { useActiveConflicts, useIgnoredActiveModConflictGroupKeys } from '@/features/mod-runtime';
import { useAppStore } from '@/app/store';
import { RandomizerModal } from '@/features/randomizer';
import { buildConflictModSetKey, ConflictModal } from '@/features/conflict-report';
import { useTranslation } from 'react-i18next';
import { toast } from '@/shared/ui/toast';

export default function LaunchBar() {
  const { t } = useTranslation(['layout']);
  const { activeGame } = useActiveGame();
  const autoCloseLauncher = useAppStore((state) => state.autoCloseLauncher);
  const workspaceView = useAppStore((state) => state.workspaceView);
  const [isLaunching, setIsLaunching] = useState(false);
  const [randomizerOpen, setRandomizerOpen] = useState(false);
  const [conflictOpen, setConflictOpen] = useState(false);
  const [moreMenuTarget, setMoreMenuTarget] = useState<HTMLElement | null>(null);

  const { data: conflicts } = useActiveConflicts();
  const ignoredConflictGroups = useIgnoredActiveModConflictGroupKeys();
  const ignoredGroupKeys = useMemo(
    () => new Set(ignoredConflictGroups.data ?? []),
    [ignoredConflictGroups.data],
  );
  const unresolvedConflicts = useMemo(
    () =>
      (conflicts ?? []).filter(
        (conflict) => !ignoredGroupKeys.has(buildConflictModSetKey(conflict.mod_paths)),
      ),
    [conflicts, ignoredGroupKeys],
  );
  const hasConflicts = unresolvedConflicts.length > 0;

  useEffect(() => {
    setMoreMenuTarget(document.getElementById('topbar-more-launch-portal'));
  }, []);

  const handleLaunch = async () => {
    if (!activeGame) return;
    setIsLaunching(true);

    try {
      await launchConfiguredGame(activeGame.id, autoCloseLauncher);
    } catch (e) {
      toast.error(formatAppError(e));
    } finally {
      setIsLaunching(false);
    }
  };

  if (!activeGame) return null;

  const openConflicts = () => {
    setConflictOpen(true);
  };

  return (
    <>
      <div className="flex items-center gap-2">
        {workspaceView === 'mods' && (
          <button
            type="button"
            className="btn btn-soft btn-sm h-9 min-h-9 px-3"
            onClick={() => setRandomizerOpen(true)}
            title={t('layout:launch_bar.randomizer')}
          >
            <Shuffle size={12} />
          </button>
        )}

        {hasConflicts && (
          <button
            type="button"
            className="btn btn-ghost btn-sm min-h-11 gap-1.5 px-2.5 text-warning transition-colors hover:bg-warning/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-warning focus-visible:outline-offset-2 active:translate-y-px"
            onClick={openConflicts}
            aria-label={`${t('layout:launch_bar.shared_hashes')}: ${unresolvedConflicts.length}`}
            title={t('layout:launch_bar.conflict_warning', { count: unresolvedConflicts.length })}
          >
            <AlertTriangle size={16} aria-hidden="true" />
            <span className="font-mono text-xs tabular-nums">{unresolvedConflicts.length}</span>
          </button>
        )}

        <button
          type="button"
          className="btn btn-primary btn-sm h-9 min-h-9 min-w-24 gap-2 border-0 px-4 shadow-none"
          onClick={() => void handleLaunch()}
          disabled={isLaunching}
        >
          {isLaunching ? (
            <span className="loading loading-spinner loading-sm" />
          ) : (
            <Play size={12} fill="currentColor" />
          )}
          {isLaunching ? t('layout:launch_bar.launching') : t('layout:launch_bar.play')}
        </button>
      </div>

      {moreMenuTarget &&
        createPortal(
          <div className="!block w-full min-w-0 max-w-full border-b border-base-content/10 px-2 pb-2">
            <span className="mb-1.5 block text-[10px] font-medium uppercase tracking-widest text-base-content/45">
              {t('layout:launch_bar.play')}
            </span>
            <div className="grid gap-1">
              {workspaceView === 'mods' && (
                <button
                  type="button"
                  className="flex min-h-9 w-full items-center gap-2 rounded-lg px-2.5 text-sm text-base-content/75 transition-colors hover:bg-base-content/10 hover:text-base-content"
                  onClick={(event) => {
                    event.currentTarget.blur();
                    setRandomizerOpen(true);
                  }}
                >
                  <Shuffle size={15} aria-hidden="true" />
                  {t('layout:launch_bar.randomizer')}
                </button>
              )}

              {hasConflicts && (
                <button
                  type="button"
                  className="flex min-h-11 w-full items-center gap-2 rounded-lg px-2.5 text-sm text-warning transition-colors hover:bg-warning/10 focus-visible:outline focus-visible:outline-2 focus-visible:outline-warning focus-visible:outline-offset-2"
                  onClick={(event) => {
                    event.currentTarget.blur();
                    openConflicts();
                  }}
                >
                  <AlertTriangle size={15} aria-hidden="true" />
                  <span>{t('layout:launch_bar.conflicts')}</span>
                  <span className="ml-auto rounded-full bg-warning/10 px-1.5 font-mono text-xs tabular-nums">
                    {unresolvedConflicts.length}
                  </span>
                </button>
              )}

              <button
                type="button"
                className="btn btn-primary btn-sm mt-1 h-10 min-h-10 w-full gap-2 border-0 px-4 shadow-none"
                onClick={(event) => {
                  event.currentTarget.blur();
                  void handleLaunch();
                }}
                disabled={isLaunching}
              >
                {isLaunching ? (
                  <span className="loading loading-spinner loading-sm" />
                ) : (
                  <Play size={13} fill="currentColor" />
                )}
                {isLaunching ? t('layout:launch_bar.launching') : t('layout:launch_bar.play')}
              </button>
            </div>
          </div>,
          moreMenuTarget,
        )}

      <RandomizerModal
        open={randomizerOpen}
        onClose={() => setRandomizerOpen(false)}
        gameId={activeGame.id}
      />

      <ConflictModal
        open={conflictOpen}
        onClose={() => setConflictOpen(false)}
        conflicts={conflicts || []}
        gameId={activeGame.id}
        ignoredGroupKeys={ignoredGroupKeys}
        isIgnoredGroupsLoading={ignoredConflictGroups.isLoading}
        ignoredGroupsError={ignoredConflictGroups.error}
        onRetryIgnoredGroups={() => void ignoredConflictGroups.refetch()}
      />
    </>
  );
}
