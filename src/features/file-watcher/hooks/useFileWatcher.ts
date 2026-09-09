import { useCallback, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { QueryClient } from '@tanstack/react-query';
import i18next from 'i18next';
import { useAppStore } from '@/app/store';
import type { GameConfig } from '@/entities/game';
import {
  commands,
  type DiskReconcileReason,
  type DiskReconcileResult,
} from '../../../shared/api/tauri/bindings';
import { formatAppError } from '../../../shared/lib/appError';
import { publishDiskReconcileRefresh } from '../utils/reconcileRefresh';
import {
  buildDiskReconcilePathRewrites,
  clearStaleSelections,
  isPreviewAffected,
} from '../utils/reconcileSelection';
import { maybeShowExternalChangeToast } from '../utils/reconcileToast';
import { applyWorkspacePathRewrites } from '@/features/workspace-runtime/@x/file-watcher';
import { toast } from '@/shared/ui/toast';
import {
  openFolderConflictManagerDialog,
  openRenameConfirmationDialog,
} from '@/features/workspace-runtime/@x/file-watcher';
import { useWatcherLifecycle } from '../utils/watcherLifecycle';
import { workspaceKeys } from '@/features/workspace-runtime/@x/file-watcher';
import { useDiskReconcileProgress } from '../utils/reconcileProgress';
import { isDuplicateWatcherError, type WatchErrorPayload } from '../utils/watcherError';

export { useWatcherLifecycle } from '../utils/watcherLifecycle';

const MODS_VIEW_SYNC_TTL_MS = 5_000;
const WINDOW_REFOCUS_MIN_BLUR_MS = 750;
const AUTO_OPEN_REPORT_MAX_GAMES = 32;
const RUNTIME_WARNING_DEDUPE_MS = 3_000;
const autoOpenedConflictReportByGame = new Map<string, string>();
const autoOpenedRenameReportByGame = new Map<string, string>();
const lastRuntimeWarningByGame = new Map<string, { key: string; at: number }>();

function setBoundedMapEntry<K, V>(map: Map<K, V>, key: K, value: V, maxEntries: number) {
  if (map.has(key)) {
    map.delete(key);
  }
  while (map.size >= maxEntries) {
    const oldestKey = map.keys().next().value as K | undefined;
    if (oldestKey === undefined) {
      break;
    }
    map.delete(oldestKey);
  }
  map.set(key, value);
}

function maybeShowRuntimeEffectsWarning(result: DiskReconcileResult) {
  const warning = result.warnings.find((entry) => entry.kind === 'RuntimeEffectsPending');
  if (!warning) {
    return;
  }

  const now = Date.now();
  const warningKey = `${warning.kind}:${warning.message}`;
  const previous = lastRuntimeWarningByGame.get(result.game_id);
  if (previous?.key === warningKey && now - previous.at < RUNTIME_WARNING_DEDUPE_MS) {
    return;
  }

  setBoundedMapEntry(
    lastRuntimeWarningByGame,
    result.game_id,
    { key: warningKey, at: now },
    AUTO_OPEN_REPORT_MAX_GAMES,
  );
  const fallback = 'Disk changes were applied, but runtime refresh is still pending.';
  toast.warning(
    i18next.t('common:reconcile.runtime_effects_pending', {
      defaultValue: fallback,
    }) || fallback,
  );
}

interface QueuedDiskReconcileRefresh {
  reason: DiskReconcileReason;
  forceFull: boolean;
  skipSyncCheck: boolean;
}

export function applyDiskReconcileResult(
  result: DiskReconcileResult,
  queryClient: QueryClient,
  activeGame: GameConfig | null,
  presentRepairDialogs = true,
) {
  // Disk Reconcile owns filesystem truth and global runtime refresh for disk-backed changes.
  const appStore = useAppStore.getState();
  if (result.status === 'SourceUnavailable') {
    appStore.setFolderConflicts(result.game_id, []);
    appStore.setRenameConfirmations(result.game_id, []);
    appStore.setDiskSourceUnavailable(
      result.game_id,
      result.error_message ?? 'Mods folder is unavailable',
    );
    return;
  }

  maybeShowRuntimeEffectsWarning(result);

  const hasFolderConflicts = result.status === 'AppliedWithFolderConflicts';
  if (hasFolderConflicts) {
    appStore.setDiskSourceUnavailable(result.game_id, null);
    appStore.setFolderConflicts(result.game_id, result.folder_conflicts);
    appStore.setRenameConfirmations(result.game_id, []);
    const reportKey = result.folder_conflicts
      .map((group) => {
        const candidates = group.candidates
          .map((candidate) => `${candidate.path}:${candidate.is_enabled}`)
          .sort()
          .join(',');
        return `${group.group_id}:${candidates}`;
      })
      .sort()
      .join('|');
    if (
      presentRepairDialogs &&
      activeGame?.id === result.game_id &&
      autoOpenedConflictReportByGame.get(result.game_id) !== reportKey
    ) {
      setBoundedMapEntry(
        autoOpenedConflictReportByGame,
        result.game_id,
        reportKey,
        AUTO_OPEN_REPORT_MAX_GAMES,
      );
      openFolderConflictManagerDialog();
    }
  }

  if (result.status === 'NeedsRenameConfirmation') {
    appStore.setDiskSourceUnavailable(result.game_id, null);
    appStore.setFolderConflicts(result.game_id, []);
    appStore.setRenameConfirmations(result.game_id, result.rename_confirmations);
    const reportKey = result.rename_confirmations
      .map(
        (group) =>
          `${group.group_id}:${group.previous_paths.slice().sort().join(',')}:${group.current_paths.slice().sort().join(',')}`,
      )
      .sort()
      .join('|');
    if (
      presentRepairDialogs &&
      activeGame?.id === result.game_id &&
      autoOpenedRenameReportByGame.get(result.game_id) !== reportKey
    ) {
      setBoundedMapEntry(
        autoOpenedRenameReportByGame,
        result.game_id,
        reportKey,
        AUTO_OPEN_REPORT_MAX_GAMES,
      );
      openRenameConfirmationDialog();
    }
    return;
  }

  appStore.setDiskSourceUnavailable(result.game_id, null);
  if (!hasFolderConflicts) {
    appStore.setFolderConflicts(result.game_id, []);
  }
  appStore.setRenameConfirmations(result.game_id, []);
  if (!hasFolderConflicts) {
    autoOpenedConflictReportByGame.delete(result.game_id);
  }
  autoOpenedRenameReportByGame.delete(result.game_id);
  appStore.setDiskReconcileTimestamp(result.game_id, Date.now());
  if (result.reason === 'StartupBoot') {
    void queryClient.invalidateQueries({
      queryKey: workspaceKeys.all,
      refetchType: 'active',
    });
  }
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
  const setDiskReconcileProgress = useAppStore((state) => state.setDiskReconcileProgress);
  const inFlightRef = useRef(false);
  const queuedRefreshRef = useRef<QueuedDiskReconcileRefresh | null>(null);
  const lastModsViewSyncKeyRef = useRef<string | null>(null);
  const hydratedModsViewByGameRef = useRef<Record<string, boolean>>({});
  const requiresFullReconcileByGameRef = useRef<Record<string, boolean>>({});
  const lastWindowBlurAtRef = useRef<number>(0);
  const lastActiveGameIdRef = useRef<string | null>(activeGame?.id ?? null);

  useWatcherLifecycle(activeGame);
  useDiskReconcileProgress(activeGame?.id ?? null);

  const markGameHydrated = useCallback((gameId: string) => {
    hydratedModsViewByGameRef.current[gameId] = true;
    requiresFullReconcileByGameRef.current[gameId] = false;
    lastWindowBlurAtRef.current = 0;
  }, []);

  const recordReconcileOutcome = useCallback(
    (result: DiskReconcileResult) => {
      if (result.status === 'Applied' || result.status === 'AppliedWithFolderConflicts') {
        markGameHydrated(result.game_id);
        return;
      }

      hydratedModsViewByGameRef.current[result.game_id] = false;
      requiresFullReconcileByGameRef.current[result.game_id] = true;
    },
    [markGameHydrated],
  );

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
          applyDiskReconcileResult(result, queryClient, activeGame, workspaceView === 'mods');
          recordReconcileOutcome(result);
        } catch (error) {
          console.error('[DiskReconcile] Refresh failed:', error);
          requiresFullReconcileByGameRef.current[gameId] = true;
          setDiskReconcileProgress(gameId, null);
          markDiskReconcilePending(gameId, true);
          toast.warning(
            i18next.t('grid:banners.disk_sync_failed', { error: formatAppError(error) }),
          );
        } finally {
          inFlightRef.current = false;
          nextRefresh = queuedRefreshRef.current;
          queuedRefreshRef.current = null;
        }
      }
    },
    [
      activeGame,
      markDiskReconcilePending,
      queryClient,
      recordReconcileOutcome,
      setDiskReconcileProgress,
      shouldSync,
      workspaceView,
    ],
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

      applyDiskReconcileResult(event.payload, queryClient, activeGame, workspaceView === 'mods');
      recordReconcileOutcome(event.payload);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [activeGame?.id, activeGame, queryClient, recordReconcileOutcome, workspaceView]);

  useEffect(() => {
    if (!activeGame?.id) {
      return;
    }

    const gameId = activeGame.id;
    const unlistenPromise = listen<WatchErrorPayload>('mod_watch:event', (event) => {
      if (event.payload.type !== 'Error' || event.payload.game_id !== gameId) {
        return;
      }

      console.error('[Watcher] error:', event.payload.error, event.payload.path);
      const now = Date.now();
      if (isDuplicateWatcherError(event.payload, now)) {
        markDiskReconcilePending(gameId, true);
        return;
      }
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
