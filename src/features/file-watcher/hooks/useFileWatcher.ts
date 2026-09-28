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
import { publishRuntimeDescriptor } from '@/shared/lib/queryRefresh';
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
  buildRuntimeMutationDescriptor,
  formatCollectionReferenceImpact,
  openRenameConfirmationDialog,
  workspaceKeys,
} from '@/features/workspace-runtime/@x/file-watcher';
import { useDiskReconcileProgress } from '../utils/reconcileProgress';
import { isDuplicateWatcherError, type WatchErrorPayload } from '../utils/watcherError';
import { reconcileModViewerExternalReviews } from '@/features/mod-runtime/@x/file-watcher';
import { joinModPath } from '../utils/pathUtils';

const WINDOW_REFOCUS_MIN_BLUR_MS = 750;
const AUTO_OPEN_REPORT_MAX_GAMES = 32;
const RECONCILE_WARNING_DEDUPE_MS = 30_000;
const autoOpenedRenameReportByGame = new Map<string, string>();
const lastReconcileWarningByGame = new Map<string, { key: string; at: number }>();

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

function maybeShowReconcileWarning(result: DiskReconcileResult) {
  const warning =
    result.warnings.find((entry) => entry.kind === 'AuthorityPending') ??
    result.warnings.find((entry) => entry.kind === 'RuntimeEffectsPending');
  if (!warning) {
    lastReconcileWarningByGame.delete(result.game_id);
    return;
  }

  const now = Date.now();
  const warningKey = `${warning.kind}:${warning.message}`;
  const previous = lastReconcileWarningByGame.get(result.game_id);
  if (previous?.key === warningKey && now - previous.at < RECONCILE_WARNING_DEDUPE_MS) {
    return;
  }

  setBoundedMapEntry(
    lastReconcileWarningByGame,
    result.game_id,
    { key: warningKey, at: now },
    AUTO_OPEN_REPORT_MAX_GAMES,
  );
  const authorityPending = warning.kind === 'AuthorityPending';
  const fallback = authorityPending
    ? 'Disk changes were applied, but indexing validation is still pending.'
    : 'Disk changes were applied, but runtime refresh is still pending.';
  toast.warning(
    i18next.t(
      authorityPending
        ? 'common:reconcile.authority_pending'
        : 'common:reconcile.runtime_effects_pending',
      {
        defaultValue: fallback,
      },
    ) || fallback,
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
  const previousRevision = appStore.diskReconcileByGame[result.game_id]?.revision ?? 0;
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
    maybeShowReconcileWarning(result);
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
  if (result.scan_scope === 'None' && result.reconcile_revision > previousRevision) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor([
        'objectStructure',
        'folderMetadataPreview',
        'thumbnailOnly',
        'collectionsCatalog',
        'dashboardKeybindings',
        'conflictsOnly',
      ]),
      'active',
    );
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
  const watcherFailureEpochByGameRef = useRef<Record<string, number>>({});
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
      if (previous && candidate && previous.gameId === candidate.gameId) {
        hydratedModsViewByGameRef.current[previous.gameId] = false;
        requiresFullReconcileByGameRef.current[previous.gameId] = true;
      }
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
    (result: DiskReconcileResult, startedAtWatcherFailureEpoch?: number) => {
      if (result.status === 'Applied' || result.status === 'AppliedWithFolderConflicts') {
        const currentFailureEpoch = watcherFailureEpochByGameRef.current[result.game_id] ?? 0;
        if (
          (startedAtWatcherFailureEpoch !== undefined &&
            startedAtWatcherFailureEpoch !== currentFailureEpoch) ||
          (startedAtWatcherFailureEpoch === undefined &&
            requiresFullReconcileByGameRef.current[result.game_id])
        ) {
          hydratedModsViewByGameRef.current[result.game_id] = false;
          requiresFullReconcileByGameRef.current[result.game_id] = true;
          markDiskReconcilePending(result.game_id, true);
          return;
        }
        markGameHydrated(result.game_id);
        return;
      }

      hydratedModsViewByGameRef.current[result.game_id] = false;
      requiresFullReconcileByGameRef.current[result.game_id] = true;
    },
    [markDiskReconcilePending, markGameHydrated],
  );

  useEffect(() => {
    if (
      activeGame?.id &&
      gameActivation?.phase === 'ready' &&
      gameActivation.reconcile_revision !== null
    ) {
      markGameHydrated(activeGame.id);
    }
  }, [activeGame?.id, gameActivation?.phase, gameActivation?.reconcile_revision, markGameHydrated]);

  const shouldSync = useCallback(
    (gameId: string, fullRepairRequired: boolean) =>
      fullRepairRequired || (useAppStore.getState().diskReconcileByGame[gameId]?.pending ?? false),
    [],
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
      };
      if (inFlightRef.current !== null) {
        const queued = queuedRefreshRef.current;
        queuedRefreshRef.current =
          queued && isSameRefreshContext(queued, requestedRefresh)
            ? {
                ...requestedRefresh,
                forceFull: requestedRefresh.forceFull || queued.forceFull,
              }
            : requestedRefresh;
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

        const fullRepairRequired =
          currentRefresh.forceFull ||
          (requiresFullReconcileByGameRef.current[currentRefresh.gameId] ?? false) ||
          !hydratedModsViewByGameRef.current[currentRefresh.gameId];
        const isViewAuthorityCheck =
          currentRefresh.reason === 'ModsViewEntered' ||
          currentRefresh.reason === 'WindowRefocused';
        const needsSync = shouldSync(currentRefresh.gameId, fullRepairRequired);
        const silentAuthorityCheck = isViewAuthorityCheck && !needsSync;
        if (!needsSync && !silentAuthorityCheck) {
          const queued = queuedRefreshRef.current;
          queuedRefreshRef.current = null;
          nextRefresh =
            queued && isSameRefreshContext(queued, activeContextRef.current) ? queued : null;
          continue;
        }

        inFlightRef.current = currentRefresh;
        if (!silentAuthorityCheck) {
          markDiskReconcilePending(currentRefresh.gameId, true);
        }
        const startedAtWatcherFailureEpoch =
          watcherFailureEpochByGameRef.current[currentRefresh.gameId] ?? 0;

        try {
          // Disk Reconcile only. This path must never trigger the Deep Match Scanner.
          const result = await commands.reconcileDiskStateCmd(
            currentRefresh.gameId,
            currentRefresh.reason,
            null,
            fullRepairRequired,
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
            appliesToActiveContext ||
            activeContextRef.current?.gameId !== currentRefresh.gameId
          ) {
            const applied = applyDiskReconcileResult(
              result,
              queryClient,
              activeGameForResult,
              appliesToActiveContext && workspaceViewRef.current === 'mods',
              result.game_id === currentRefresh.gameId ? currentRefresh.modsPathKey : undefined,
            );
            if (applied) {
              recordReconcileOutcome(result, startedAtWatcherFailureEpoch);
            } else if (
              !silentAuthorityCheck &&
              result.reconcile_revision ===
                useAppStore.getState().diskReconcileByGame[result.game_id]?.revision &&
              (result.status === 'Applied' || result.status === 'AppliedWithFolderConflicts') &&
              startedAtWatcherFailureEpoch ===
                (watcherFailureEpochByGameRef.current[result.game_id] ?? 0)
            ) {
              markGameHydrated(result.game_id);
              markDiskReconcilePending(result.game_id, false);
            }
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
      markGameHydrated,
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

    let effectActive = true;
    let startupResultsDrained = false;
    const eventsDuringStartupDrain: DiskReconcileResult[] = [];
    const registeredContext = activeContextRef.current;
    const applyResult = (result: DiskReconcileResult) => {
      const currentContext = activeContextRef.current;
      const appliesToActiveContext = isSameRefreshContext(registeredContext, currentContext);
      if (!appliesToActiveContext && currentContext?.gameId === result.game_id) {
        return;
      }
      const currentGame = appliesToActiveContext ? activeGameRef.current : null;
      if (
        applyDiskReconcileResult(
          result,
          queryClient,
          currentGame,
          appliesToActiveContext && workspaceViewRef.current === 'mods',
          activeGame.mod_path,
        )
      ) {
        recordReconcileOutcome(result);
      }
    };
    const unlistenPromise = listen<DiskReconcileResult>('disk_reconcile:result', (event) => {
      if (event.payload.game_id !== activeGame.id) {
        return;
      }

      if (!startupResultsDrained) {
        eventsDuringStartupDrain.push(event.payload);
        return;
      }
      applyResult(event.payload);
    });
    void unlistenPromise
      .then((unlisten) => {
        if (!effectActive) {
          unlisten();
          return;
        }
        const startupResults = useAppStore
          .getState()
          .takeStartupDiskReconcileResults(activeGame.id);
        const resultsByRevision = new Map<number, DiskReconcileResult>();
        for (const result of [...startupResults, ...eventsDuringStartupDrain]) {
          resultsByRevision.set(result.reconcile_revision, result);
        }
        startupResultsDrained = true;
        for (const result of [...resultsByRevision.values()].sort(
          (left, right) => left.reconcile_revision - right.reconcile_revision,
        )) {
          applyResult(result);
        }
      })
      .catch((error: unknown) => {
        console.error('Failed to register disk reconcile listener', error);
      });

    return () => {
      effectActive = false;
      void unlistenPromise
        .then((unlisten) => unlisten())
        .catch((error: unknown) => {
          console.error('Failed to remove disk reconcile listener', error);
        });
    };
  }, [activeGame?.id, activeGame, queryClient, recordReconcileOutcome]);

  useEffect(() => {
    if (!activeGame?.id) {
      return;
    }

    const gameId = activeGame.id;
    const registeredContext = activeContextRef.current;
    const unlistenPromise = listen<WatchErrorPayload>('mod_watch:event', (event) => {
      if (event.payload.type !== 'Error' || event.payload.game_id !== gameId) {
        return;
      }
      if (
        activeContextRef.current?.gameId === gameId &&
        !isSameRefreshContext(registeredContext, activeContextRef.current)
      ) {
        return;
      }

      console.error('[Watcher] error:', event.payload.error, event.payload.path);
      const now = Date.now();
      watcherFailureEpochByGameRef.current[gameId] =
        (watcherFailureEpochByGameRef.current[gameId] ?? 0) + 1;
      requiresFullReconcileByGameRef.current[gameId] = true;
      markDiskReconcilePending(gameId, true);
      if (activeContextRef.current?.gameId === gameId) {
        void runRefresh('WatcherBatch', true);
      }
      if (isDuplicateWatcherError(event.payload, now)) {
        return;
      }
      toast.warning(i18next.t('common:watcher.error', { error: event.payload.error }));
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, [activeGame?.id, activeGame?.mod_path, markDiskReconcilePending, runRefresh]);

  useEffect(() => {
    if (!activeGame?.id) {
      return;
    }

    const registeredContext = activeContextRef.current;
    const unlistenFocusPromise = listen('tauri://focus', () => {
      if (
        workspaceViewRef.current !== 'mods' ||
        !isSameRefreshContext(registeredContext, activeContextRef.current)
      ) {
        return;
      }

      const gameId = registeredContext?.gameId;
      if (!gameId) {
        return;
      }
      const pending = useAppStore.getState().diskReconcileByGame[gameId]?.pending ?? false;
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
  }, [activeGame?.id, activeGame?.mod_path, runRefresh]);
}
