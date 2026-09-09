import { useEffect, useRef } from 'react';
import { commands } from '../../../shared/api/tauri/bindings';
import { useAppStore } from '@/app/store';
import type { GameConfig } from '@/entities/game';

function stopWatcherAfterUnmount(generationRef: { current: number }, generation: number) {
  queueMicrotask(() => {
    if (generationRef.current !== generation) return;
    void commands.stopWatcher().catch((error: unknown) => {
      console.error('[DiskReconcile] Failed to stop watcher:', error);
    });
  });
}

export function useWatcherLifecycle(activeGame: GameConfig | null) {
  const diskSourceUnavailable = useAppStore((state) =>
    Boolean(activeGame?.id && state.diskReconcileByGame[activeGame.id]?.unavailable),
  );
  const lifecycleGeneration = useRef(0);
  const mountGeneration = useRef(0);

  useEffect(() => {
    const generation = ++lifecycleGeneration.current;
    if (!activeGame?.mod_path || !activeGame?.id || diskSourceUnavailable) {
      void commands.stopWatcher().catch((error: unknown) => {
        console.error('[DiskReconcile] Failed to stop watcher:', error);
      });
      return;
    }

    void commands.startWatcher(activeGame.mod_path, activeGame.id).catch((error: unknown) => {
      if (lifecycleGeneration.current !== generation) return;
      console.error('[DiskReconcile] Failed to start watcher:', error);
    });
  }, [activeGame?.id, activeGame?.mod_path, diskSourceUnavailable]);

  useEffect(() => {
    const generation = ++mountGeneration.current;
    return () => stopWatcherAfterUnmount(mountGeneration, generation);
  }, []);
}
