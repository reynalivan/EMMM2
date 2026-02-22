import { Play, Shuffle, AlertTriangle } from 'lucide-react';
import { useState, useEffect } from 'react';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useActiveConflicts } from '../../hooks/useFolders';
import { invoke } from '@tauri-apps/api/core';
import RandomizerModal from '../Randomizer/RandomizerModal';
import ConflictModal from '../ConflictReport/ConflictModal';
import ConflictToast from '../scanner/ConflictToast';

export default function LaunchBar() {
  const { activeGame } = useActiveGame();
  const [isLaunching, setIsLaunching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [randomizerOpen, setRandomizerOpen] = useState(false);
  const [conflictOpen, setConflictOpen] = useState(false);
  const [showToast, setShowToast] = useState(false);

  const { data: conflicts } = useActiveConflicts();
  const hasConflicts = conflicts && conflicts.length > 0;

  useEffect(() => {
    if (hasConflicts) setShowToast(true);
    else setShowToast(false);
  }, [hasConflicts]);

  const handleLaunch = async () => {
    if (!activeGame) return;
    setIsLaunching(true);
    setError(null);

    try {
      await invoke('launch_game', { gameId: activeGame.id });
    } catch (e) {
      setError(String(e));
      setTimeout(() => setError(null), 5000);
    } finally {
      setIsLaunching(false);
    }
  };

  if (!activeGame) return null;

  return (
    <div className="p-3 border-t border-base-300/20 bg-base-200/30 flex flex-col gap-2">
      {error && (
        <div className="alert alert-error text-xs py-1 px-2 rounded-md flex items-center gap-2">
          <AlertTriangle size={12} />
          <span className="truncate">{error}</span>
        </div>
      )}
      <div className="flex items-center gap-2">
        <button
          className="btn btn-neutral btn-sm px-3"
          onClick={() => setRandomizerOpen(true)}
          title="Randomizer (Gacha)"
        >
          <Shuffle size={12} />
        </button>

        {hasConflicts && (
          <div className="relative">
            <button
              className="btn btn-warning btn-sm shadow-lg animate-pulse"
              onClick={() => {
                setConflictOpen(true);
                setShowToast(false);
              }}
              title={`${conflicts.length} Shader Conflicts Detected`}
            >
              <AlertTriangle size={16} />
              <span className="hidden sm:inline">Conflicts</span>
            </button>
            {showToast && (
              <ConflictToast conflicts={conflicts} onDismiss={() => setShowToast(false)} />
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
          {isLaunching ? 'Launching...' : 'Play'}
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
      />
    </div>
  );
}
