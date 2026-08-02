import { useCallback, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { QueryClient } from '@tanstack/react-query';
import i18next from 'i18next';
import { useAppStore } from '../../stores/useAppStore';
import type { GameConfig } from '../../types/game';
import { commands, type DiskReconcileReason, type DiskReconcileResult } from '../../lib/bindings';
import { publishDiskReconcileRefresh } from './reconcileRefresh';
import {
  buildDiskReconcilePathRewrites,
  clearStaleSelections,
  isPreviewAffected,
} from './reconcileSelection';
import { maybeShowExternalChangeToast } from './reconcileToast';
import { applyWorkspacePathRewrites } from '../workspace-runtime/optimistic/workspaceViewModelRewrite';
import { toast } from '../../stores/useToastStore';

interface WatchErrorPayload {
  type: string;
  error: string;
  path?: string | null;
}

const MODS_VIEW_SYNC_TTL_MS = 5_000;
const WINDOW_REFOCUS_MIN_BLUR_MS = 750;

interface QueuedDiskReconcileRefresh {
  reason: DiskReconcileReason;
  forceFull: boolean;
  skipSyncCheck: boolean;
}

export function useWatcherLifecycle(activeGame: GameConfig | null) {
  // Recovery: when the mods folder disappears the watcher dies with it.
  // Re-running the effect once the source is available again restarts it.
  const diskSourceUnavailable = useAppStore((state) =>
    Boolean(activeGame?.id && state.diskReconcileByGame[activeGame.id]?.unavailable),
  );

  useEffect(() => {
    let cancelled = false;

    const init = async () => {
      await commands.stopWatcher().catch(() => {});
      if (cancelled) {
        return;
      }

      if (!activeGame?.mod_path || !activeGame?.id || diskSourceUnavailable) {
        return;
      }

      await commands.startWatcher(activeGame.mod_path, activeGame.id).catch((error: unknown) => {
        console.error('[DiskReconcile] Failed to start watcher:', error);
      });
    };

    void init();

    return () => {
      cancelled = true;
      commands.stopWatcher().catch(() => {});
    };
  }, [activeGame?.id, activeGame?.mod_path, diskSourceUnavailable]);
}

export function applyDiskReconcileResult(
  result: DiskReconcileResult,
  queryClient: QueryClient,
  activeGame: GameConfig | null,
) {
  // Disk Reconcile owns filesystem truth and global runtime refresh for disk-backed changes.
  const appStore = useAppStore.getState();
  if (result.status === 'SourceUnavailable') {
    appStore.setDiskSourceUnavailable(
      result.game_id,
      result.error_message ?? 'Mods folder is unavailable',
    );
    return;
  }

  appStore.setDiskSourceUnavailable(result.game_id, null);
  appStore.setDiskReconcileTimestamp(result.game_id, Date.now());
  applyWorkspacePathRewrites(buildDiskReconcilePathRewrites(result, activeGame), 'disk_reconcile');
  clearStaleSelections(result, activeGame);
  publishDiskReconcileRefresh(queryClient, result, isPreviewAffected(result, activeGame));

  maybeShowExternalChangeToast(result);
}

export function useDiskReconcileCoordinator(
  activeGame: GameConfig | null,
  queryClient: QueryClient,
) {
  const workspaceView = useAppStore((state) => state.workspaceView);
  const diskReconcileByGame = useAppStore((state) => state.diskReconcileByGame);
  const markDiskReconcilePending = useAppStore((state) => state.markDiskReconcilePending);
  const inFlightRef = useRef(false);
  const queuedRefreshRef = useRef<QueuedDiskReconcileRefresh | null>(null);
  const lastModsViewSyncKeyRef = useRef<string | null>(null);
  const hydratedModsViewByGameRef = useRef<Record<string, boolean>>({});
  const requiresFullReconcileByGameRef = useRef<Record<string, boolean>>({});
  const lastWindowBlurAtRef = useRef<number>(0);
  const lastActiveGameIdRef = useRef<string | null>(activeGame?.id ?? null);

  useWatcherLifecycle(activeGame);

  const markGameHydrated = useCallback((gameId: string) => {
    hydratedModsViewByGameRef.current[gameId] = true;
    requiresFullReconcileByGameRef.current[gameId] = false;
    lastWindowBlurAtRef.current = 0;
  }, []);

  const shouldSync = useCallback(
    (gameId: string, forceFull: boolean) => {
      if (forceFull) {
        return true;
      }

      if (requiresFullReconcileByGameRef.current[gameId]) {
        return true;
      }

      if (!hydratedModsViewByGameRef.current[gameId]) {
        return true;
      }

      const entry = diskReconcileByGame[gameId];
      if (entry?.pending) {
        return true;
      }

      return Date.now() - (entry?.at ?? 0) > MODS_VIEW_SYNC_TTL_MS;
    },
    [diskReconcileByGame],
  );

  const runRefresh = useCallback(
    async (reason: DiskReconcileReason, forceFull: boolean) => {
      if (!activeGame?.id) {
        return;
      }

      const gameId = activeGame.id;
      if (inFlightRef.current) {
        const queued = queuedRefreshRef.current;
        queuedRefreshRef.current = {
          reason,
          forceFull: forceFull || (queued?.forceFull ?? false),
          skipSyncCheck: true,
        };
        markDiskReconcilePending(gameId, true);
        return;
      }

      let nextRefresh: QueuedDiskReconcileRefresh | null = {
        reason,
        forceFull,
        skipSyncCheck: false,
      };

      while (nextRefresh) {
        const currentRefresh = nextRefresh;

        if (!currentRefresh.skipSyncCheck && !shouldSync(gameId, currentRefresh.forceFull)) {
          return;
        }

        inFlightRef.current = true;
        markDiskReconcilePending(gameId, true);

        try {
          // Disk Reconcile only. This path must never trigger the Deep Match Scanner.
          const result = await commands.reconcileDiskStateCmd(
            gameId,
            currentRefresh.reason,
            null,
            currentRefresh.forceFull,
          );
          applyDiskReconcileResult(result, queryClient, activeGame);
          markGameHydrated(gameId);
        } catch (error) {
          console.error('[DiskReconcile] Refresh failed:', error);
        } finally {
          inFlightRef.current = false;
          nextRefresh = queuedRefreshRef.current;
          queuedRefreshRef.current = null;
        }
      }
    },
    [activeGame, markDiskReconcilePending, markGameHydrated, queryClient, shouldSync],
  );

  useEffect(() => {
    const currentGameId = activeGame?.id ?? null;
    const previousGameId = lastActiveGameIdRef.current;
    if (!currentGameId) {
      lastActiveGameIdRef.current = currentGameId;
      return;
    }

    if (previousGameId && previousGameId !== currentGameId) {
      requiresFullReconcileByGameRef.current[currentGameId] = true;
    }

    lastActiveGameIdRef.current = currentGameId;
  }, [activeGame?.id]);

  useEffect(() => {
    const currentGameId = activeGame?.id ?? null;
    const syncKey = currentGameId ? `${workspaceView}:${currentGameId}` : null;

    if (!currentGameId || workspaceView !== 'mods') {
      lastModsViewSyncKeyRef.current = syncKey;
      return;
    }

    const previousKey = lastModsViewSyncKeyRef.current;
    lastModsViewSyncKeyRef.current = syncKey;
    if (previousKey === syncKey) {
      return;
    }

    const requiresFull = requiresFullReconcileByGameRef.current[currentGameId] ?? false;
    const isHydrated = hydratedModsViewByGameRef.current[currentGameId] ?? false;
    const reason: DiskReconcileReason = requiresFull ? 'GameSwitched' : 'ModsViewEntered';
    void runRefresh(reason, requiresFull || !isHydrated);
  }, [activeGame?.id, runRefresh, workspaceView]);

  useEffect(() => {
    if (!activeGame?.id) {
      return;
    }

    const unlistenPromise = listen<DiskReconcileResult>('disk_reconcile:result', (event) => {
      if (event.payload.game_id !== activeGame.id) {
        return;
      }

      applyDiskReconcileResult(event.payload, queryClient, activeGame);
      markGameHydrated(activeGame.id);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [activeGame?.id, activeGame, markGameHydrated, queryClient]);

  useEffect(() => {
    if (!activeGame?.id) {
      return;
    }

    const gameId = activeGame.id;
    const unlistenPromise = listen<WatchErrorPayload>('mod_watch:event', (event) => {
      if (event.payload.type !== 'Error') {
        return;
      }

      console.error('[Watcher] error:', event.payload.error, event.payload.path);
      toast.warning(i18next.t('common:watcher.error', { error: event.payload.error }));
      // Mark state dirty so the next TTL sync repairs anything missed.
      markDiskReconcilePending(gameId, true);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [activeGame?.id, markDiskReconcilePending]);

  useEffect(() => {
    if (!activeGame?.id) {
      return;
    }

    const unlistenFocusPromise = listen('tauri://focus', () => {
      if (workspaceView !== 'mods') {
        return;
      }

      const gameId = activeGame.id;
      const pending = diskReconcileByGame[gameId]?.pending ?? false;
      const requiresFull = requiresFullReconcileByGameRef.current[gameId] ?? false;
      const isHydrated = hydratedModsViewByGameRef.current[gameId] ?? false;
      const lastBlurAt = lastWindowBlurAtRef.current;
      const blurElapsed = lastBlurAt > 0 ? Date.now() - lastBlurAt : Number.POSITIVE_INFINITY;
      if (!pending && !requiresFull && isHydrated && blurElapsed < WINDOW_REFOCUS_MIN_BLUR_MS) {
        return;
      }

      void runRefresh('WindowRefocused', requiresFull);
    });
    const unlistenBlurPromise = listen('tauri://blur', () => {
      lastWindowBlurAtRef.current = Date.now();
    });

    return () => {
      unlistenFocusPromise.then((unlisten) => unlisten());
      unlistenBlurPromise.then((unlisten) => unlisten());
    };
  }, [activeGame?.id, activeGame, diskReconcileByGame, runRefresh, workspaceView]);
}
