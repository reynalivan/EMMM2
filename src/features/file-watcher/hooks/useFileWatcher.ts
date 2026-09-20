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
import { canonicalPathKey, identityPathKey, pathStartsWith } from '@/shared/lib/pathKey';
import { publishDiskReconcileRefresh } from '../utils/reconcileRefresh';
import {
  buildDiskReconcilePathRewrites,
  clearStaleSelections,
  isPreviewAffected,
} from '../utils/reconcileSelection';
import { maybeShowExternalChangeToast } from '../utils/reconcileToast';
import { toast } from '@/shared/ui/toast';
import {
  applyWorkspacePathRewrites,
  formatCollectionReferenceImpact,
  openRenameConfirmationDialog,
  workspaceKeys,
} from '@/features/workspace-runtime/@x/file-watcher';
import { useDiskReconcileProgress } from '../utils/reconcileProgress';
import { isDuplicateWatcherError, type WatchErrorPayload } from '../utils/watcherError';
import { reconcileModViewerExternalReviews } from '@/features/mod-runtime/@x/file-watcher';
import { joinModPath } from '../utils/pathUtils';

const MODS_VIEW_SYNC_TTL_MS = 5_000;
const WINDOW_REFOCUS_MIN_BLUR_MS = 750;
const AUTO_OPEN_REPORT_MAX_GAMES = 32;
const RUNTIME_WARNING_DEDUPE_MS = 30_000;
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
    lastRuntimeWarningByGame.delete(result.game_id);
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

interface DiskReconcileRefreshContext {
  gameId: string;
  modsPathKey: string;
  generation: number;
}

interface QueuedDiskReconcileRefresh extends DiskReconcileRefreshContext {
  reason: DiskReconcileReason;
  forceFull: boolean;
  skipSyncCheck: boolean;
}

function refreshContextFor(
  activeGame: GameConfig | null,
  generation: number,
): DiskReconcileRefreshContext | null {
  if (!activeGame?.id) {
    return null;
  }

  const modsPathKey = canonicalPathKey(activeGame.mod_path);
  if (!modsPathKey) {
    return null;
  }

  return { gameId: activeGame.id, modsPathKey, generation };
}

function isSameRefreshContext(
  left: DiskReconcileRefreshContext | null,
  right: DiskReconcileRefreshContext | null,
): boolean {
  return (
    left !== null &&
    right !== null &&
    left.gameId === right.gameId &&
    left.modsPathKey === right.modsPathKey &&
    left.generation === right.generation
  );
}

function isSameRefreshTarget(
  left: DiskReconcileRefreshContext | null,
  right: DiskReconcileRefreshContext | null,
): boolean {
  return (
    left !== null &&
    right !== null &&
    left.gameId === right.gameId &&
    left.modsPathKey === right.modsPathKey
  );
}

function pathsAreRelated(left: string, right: string): boolean {
  const leftKey = identityPathKey(left);
  const rightKey = identityPathKey(right);
  if (!leftKey || !rightKey) {
    return false;
  }

  return pathStartsWith(leftKey, rightKey) || pathStartsWith(rightKey, leftKey);
}

function affectedModHealthRoots(
  result: DiskReconcileResult,
  modsPath: string | null | undefined,
): string[] {
  const roots = new Set<string>();
  const add = (relativePath: string) => {
    if (!relativePath.trim()) {
      return;
    }
    roots.add(relativePath);
    if (modsPath) {
      roots.add(joinModPath(modsPath, relativePath));
    }
  };

  result.changed_roots.forEach(add);
  result.thumbnail_roots.forEach(add);
  result.cleared_selection_paths.forEach(add);
  result.path_updates.forEach((update) => {
    add(update.from);
    add(update.to);
  });
  return [...roots];
}

function invalidateAffectedModHealthReports(
  queryClient: QueryClient,
  result: DiskReconcileResult,
  modsPath: string | null | undefined,
): void {
  const hasModsPathContext = Boolean(canonicalPathKey(modsPath));
  const roots = affectedModHealthRoots(result, modsPath);
  void queryClient.invalidateQueries({
    predicate: (query) => {
      const [domain, kind, gameId, folderPath] = query.queryKey;
      if (domain !== 'mod-health' || kind !== 'report' || gameId !== result.game_id) {
        return false;
      }
      if (result.scan_scope === 'Full') {
        return true;
      }
      if (typeof folderPath !== 'string') {
        return true;
      }
      if (!hasModsPathContext && roots.length > 0) {
        return true;
      }
      if (roots.length === 0) {
        return result.scan_scope !== 'None';
      }
      return roots.some((root) => pathsAreRelated(root, folderPath));
    },
    refetchType: 'active',
  });
}

export function applyDiskReconcileResult(
  result: DiskReconcileResult,
  queryClient: QueryClient,
  activeGame: GameConfig | null,
  presentRepairDialogs = true,
  modsPathForHealth = activeGame?.mod_path,
): boolean {
  // Disk Reconcile owns filesystem truth and global runtime refresh for disk-backed changes.
  const appStore = useAppStore.getState();
  if (!appStore.applyFolderConflictReconcileResult(result)) {
    return false;
  }
  const appliesToActiveGame = activeGame?.id === result.game_id;

  if (result.status === 'SourceUnavailable') {
    appStore.setRenameConfirmations(result.game_id, []);
    appStore.setDiskSourceUnavailable(
      result.game_id,
      result.error_message ?? 'Mods folder is unavailable',
    );
    return true;
  }

  invalidateAffectedModHealthReports(queryClient, result, modsPathForHealth);

  if (appliesToActiveGame) {
    maybeShowRuntimeEffectsWarning(result);
  }

  if (result.status === 'NeedsRenameConfirmation') {
    appStore.setDiskSourceUnavailable(result.game_id, null);
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
    return true;
  }

  appStore.setDiskSourceUnavailable(result.game_id, null);
  appStore.setRenameConfirmations(result.game_id, []);
  autoOpenedRenameReportByGame.delete(result.game_id);
  appStore.setDiskReconcileTimestamp(result.game_id, Date.now());
  if (!appliesToActiveGame) {
    return true;
  }
  if (result.reason === 'StartupBoot') {
    void queryClient.invalidateQueries({
      queryKey: workspaceKeys.all,
      refetchType: 'active',
    });
  }
  applyWorkspacePathRewrites(buildDiskReconcilePathRewrites(result, activeGame), 'disk_reconcile');
  clearStaleSelections(result, activeGame);
  publishDiskReconcileRefresh(queryClient, result, isPreviewAffected(result, activeGame));
  void reconcileModViewerExternalReviews(
    result,
    queryClient,
    activeGame?.mod_path,
    formatCollectionReferenceImpact(result.collection_reference_impact),
  );

  maybeShowExternalChangeToast(result);
  return true;
}

export function useDiskReconcileCoordinator(
  activeGame: GameConfig | null,
  queryClient: QueryClient,
) {
  const workspaceView = useAppStore((state) => state.workspaceView);
  const diskReconcileByGame = useAppStore((state) => state.diskReconcileByGame);
  const gameActivation = useAppStore((state) =>
    activeGame?.id ? state.gameActivationByGame?.[activeGame.id] : undefined,
  );
  const markDiskReconcilePending = useAppStore((state) => state.markDiskReconcilePending);
  const setDiskReconcileProgress = useAppStore((state) => state.setDiskReconcileProgress);
  const inFlightRef = useRef<QueuedDiskReconcileRefresh | null>(null);
  const queuedRefreshRef = useRef<QueuedDiskReconcileRefresh | null>(null);
  const lastModsViewSyncKeyRef = useRef<string | null>(null);
  const hydratedModsViewByGameRef = useRef<Record<string, boolean>>({});
  const requiresFullReconcileByGameRef = useRef<Record<string, boolean>>({});
  const lastWindowBlurAtRef = useRef<number>(0);
  const activeGameRef = useRef<GameConfig | null>(activeGame);
  const workspaceViewRef = useRef(workspaceView);
  const contextGenerationRef = useRef(0);
  const activeContextRef = useRef<DiskReconcileRefreshContext | null>(null);
  const isMountedRef = useRef(true);

  useDiskReconcileProgress(activeGame?.id ?? null);

  useEffect(() => {
    activeGameRef.current = activeGame;
    workspaceViewRef.current = workspaceView;
  }, [activeGame, workspaceView]);

  useEffect(() => {
    const previous = activeContextRef.current;
    const candidate = refreshContextFor(activeGame, contextGenerationRef.current);
    if (!isSameRefreshTarget(previous, candidate)) {
      contextGenerationRef.current += 1;
    }
    const next = refreshContextFor(activeGame, contextGenerationRef.current);
    activeContextRef.current = next;

    const queued = queuedRefreshRef.current;
    if (queued && !isSameRefreshContext(queued, next)) {
      queuedRefreshRef.current = null;
    }
  }, [activeGame, activeGame?.id, activeGame?.mod_path]);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
      queuedRefreshRef.current = null;
    };
  }, []);

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

  useEffect(() => {
    if (activeGame?.id && gameActivation?.phase === 'ready') {
      markGameHydrated(activeGame.id);
    }
  }, [activeGame?.id, gameActivation?.phase, markGameHydrated]);

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
      const context = activeContextRef.current;
      if (!context || !isMountedRef.current) {
        return;
      }

      const requestedRefresh: QueuedDiskReconcileRefresh = {
        ...context,
        reason,
        forceFull,
        skipSyncCheck: false,
      };
      if (inFlightRef.current !== null) {
        const queued = queuedRefreshRef.current;
        queuedRefreshRef.current =
          queued && isSameRefreshContext(queued, requestedRefresh)
            ? {
                ...requestedRefresh,
                forceFull: requestedRefresh.forceFull || queued.forceFull,
                skipSyncCheck: true,
              }
            : {
                ...requestedRefresh,
                skipSyncCheck: true,
              };
        markDiskReconcilePending(context.gameId, true);
        return;
      }

      let nextRefresh: QueuedDiskReconcileRefresh | null = requestedRefresh;

      while (nextRefresh && isMountedRef.current) {
        const currentRefresh = nextRefresh;
        if (!isSameRefreshContext(currentRefresh, activeContextRef.current)) {
          const queued = queuedRefreshRef.current;
          queuedRefreshRef.current = null;
          nextRefresh =
            queued && isSameRefreshContext(queued, activeContextRef.current) ? queued : null;
          continue;
        }

        if (
          !currentRefresh.skipSyncCheck &&
          !shouldSync(currentRefresh.gameId, currentRefresh.forceFull)
        ) {
          return;
        }

        inFlightRef.current = currentRefresh;
        markDiskReconcilePending(currentRefresh.gameId, true);

        try {
          // Disk Reconcile only. This path must never trigger the Deep Match Scanner.
          const result = await commands.reconcileDiskStateCmd(
            currentRefresh.gameId,
            currentRefresh.reason,
            null,
            currentRefresh.forceFull,
          );
          if (!isMountedRef.current) {
            return;
          }

          const appliesToActiveContext = isSameRefreshContext(
            currentRefresh,
            activeContextRef.current,
          );
          const currentGame = activeGameRef.current;
          const activeGameForResult =
            appliesToActiveContext &&
            currentGame?.id === currentRefresh.gameId &&
            canonicalPathKey(currentGame.mod_path) === currentRefresh.modsPathKey
              ? currentGame
              : null;
          if (
            applyDiskReconcileResult(
              result,
              queryClient,
              activeGameForResult,
              appliesToActiveContext && workspaceViewRef.current === 'mods',
              result.game_id === currentRefresh.gameId ? currentRefresh.modsPathKey : undefined,
            )
          ) {
            recordReconcileOutcome(result);
          }
        } catch (error) {
          console.error('[DiskReconcile] Refresh failed:', error);
          requiresFullReconcileByGameRef.current[currentRefresh.gameId] = true;
          setDiskReconcileProgress(currentRefresh.gameId, null);
          markDiskReconcilePending(currentRefresh.gameId, true);
          if (
            isMountedRef.current &&
            isSameRefreshContext(currentRefresh, activeContextRef.current)
          ) {
            toast.warning(
              i18next.t('grid:banners.disk_sync_failed', { error: formatAppError(error) }),
            );
          }
        } finally {
          inFlightRef.current = null;
          const queued = queuedRefreshRef.current;
          queuedRefreshRef.current = null;
          nextRefresh =
            isMountedRef.current && queued && isSameRefreshContext(queued, activeContextRef.current)
              ? queued
              : null;
        }
      }
    },
    [
      markDiskReconcilePending,
      queryClient,
      recordReconcileOutcome,
      setDiskReconcileProgress,
      shouldSync,
    ],
  );

  useEffect(() => {
    const currentGameId = activeGame?.id ?? null;
    const modsPathKey = canonicalPathKey(activeGame?.mod_path);
    const syncKey =
      currentGameId && modsPathKey ? `${workspaceView}:${currentGameId}:${modsPathKey}` : null;

    if (!currentGameId || workspaceView !== 'mods') {
      lastModsViewSyncKeyRef.current = syncKey;
      return;
    }

    if (
      gameActivation?.phase === 'syncing' ||
      gameActivation?.phase === 'source_unavailable' ||
      gameActivation?.phase === 'failed'
    ) {
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
    void runRefresh('ModsViewEntered', requiresFull || !isHydrated);
  }, [activeGame?.id, activeGame?.mod_path, gameActivation?.phase, runRefresh, workspaceView]);

  useEffect(() => {
    if (!activeGame?.id) {
      return;
    }

    const unlistenPromise = listen<DiskReconcileResult>('disk_reconcile:result', (event) => {
      if (event.payload.game_id !== activeGame.id) {
        return;
      }

      if (
        applyDiskReconcileResult(event.payload, queryClient, activeGame, workspaceView === 'mods')
      ) {
        recordReconcileOutcome(event.payload);
      }
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
