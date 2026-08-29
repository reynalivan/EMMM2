import { formatAppError } from '../../shared/lib/appError';
import { Play, Shuffle, AlertTriangle } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useActiveGame } from '@/pages/dashboard/hooks/useActiveGame';
import { useActiveConflicts } from '@/widgets/mod-explorer/hooks/useFolderMutations';
import { useAppStore } from '../../app/store/useAppStore';
import { commands } from '../../shared/api/tauri/bindings';
import { exit } from '@tauri-apps/plugin-process';
import RandomizerModal from '@/features/randomizer/RandomizerModal';
import ConflictModal from '@/features/conflict-report/ConflictModal';
import ConflictToast from '@/features/scanner/components/ConflictToast';
import { useTranslation } from 'react-i18next';
import type { ConflictInfo } from '@/entities/workspace/model/scanner';

function buildConflictSignature(conflicts: ConflictInfo[]): string | null {
  if (conflicts.length === 0) return null;

  return conflicts
    .map((conflict) => {
      const evidence = conflict.evidence
        .map((item) =>
          [
            item.mod_path,
            item.source_path,
            item.section_name,
            item.condition ?? '',
            item.priority ?? '',
            item.match_first_index ?? '',
            item.shader_stage ?? '',
          ].join(':'),
        )
        .sort()
        .join(',');
      return [
        conflict.kind,
        conflict.hash,
        conflict.certainty,
        [...conflict.mod_paths].sort().join(','),
        evidence,
      ].join('|');
    })
    .sort()
    .join('||');
}

export default function LaunchBar() {
  const { t } = useTranslation(['layout']);
  const { activeGame } = useActiveGame();
  const autoCloseLauncher = useAppStore((state) => state.autoCloseLauncher);
  const [isLaunching, setIsLaunching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [randomizerOpen, setRandomizerOpen] = useState(false);
  const [conflictOpen, setConflictOpen] = useState(false);
  const [dismissedConflictSignature, setDismissedConflictSignature] = useState<string | null>(null);

  const { data: conflicts } = useActiveConflicts();
  const hasConflicts = conflicts && conflicts.length > 0;
  const conflictSignature = useMemo(() => buildConflictSignature(conflicts ?? []), [conflicts]);
  const showToast = !!hasConflicts && conflictSignature !== dismissedConflictSignature;
  const dismissCurrentConflicts = () => setDismissedConflictSignature(conflictSignature);

  const handleLaunch = async () => {
    if (!activeGame) return;
    setIsLaunching(true);
    setError(null);

    try {
      await commands.launchGame(activeGame.id);

      if (autoCloseLauncher) {
        await exit(0);
      }
    } catch (e) {
      setError(formatAppError(e));
      setTimeout(() => setError(null), 5000);
    } finally {
      setIsLaunching(false);
    }
  };

  if (!activeGame) return null;

  return (
    // Renders inside the top bar — no bottom-bar chrome (border-t / panel bg).
    <div className="flex flex-col gap-2">
      {error && (
        <div className="alert alert-error text-xs py-1 px-2 rounded-md flex items-center gap-2">
          <AlertTriangle size={12} />
          <span className="truncate">{error}</span>
        </div>
      )}
      <div className="flex items-center gap-2">
        <button
          className="btn btn-soft btn-sm px-3"
          onClick={() => setRandomizerOpen(true)}
          title={t('layout:launch_bar.randomizer')}
        >
          <Shuffle size={12} />
        </button>

        {hasConflicts && (
          <div className="relative">
            <button
              className="btn btn-warning btn-sm shadow-lg animate-pulse"
              onClick={() => {
                setConflictOpen(true);
                dismissCurrentConflicts();
              }}
              title={t('layout:launch_bar.conflict_toast', { count: conflicts.length })}
            >
              <AlertTriangle size={16} />
              <span className="hidden sm:inline">{t('layout:launch_bar.conflicts')}</span>
            </button>
            {showToast && (
              <ConflictToast conflicts={conflicts} onDismiss={dismissCurrentConflicts} />
            )}
          </div>
        )}

        <button
          className="btn btn-primary btn-sm flex-1 gap-2 shadow-lg shadow-primary/20 hover:shadow-primary/40 transition-all"
          onClick={handleLaunch}
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
      />
    </div>
  );
}
