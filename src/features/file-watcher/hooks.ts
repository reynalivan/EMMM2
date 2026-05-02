import { useCallback, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { QueryClient } from '@tanstack/react-query';
import { useAppStore } from '../../stores/useAppStore';
import { toast } from '../../stores/useToastStore';
import type { GameConfig } from '../../types/game';
import { commands, type DiskReconcileReason, type DiskReconcileResult } from '../../lib/bindings';
import { isSameOrDescendantPath, joinModPath, normalizePath, rewritePath } from './pathUtils';
import { publishRuntimeDescriptor } from '../runtime-sync/queryRefresh';
import { buildRuntimeMutationDescriptor } from '../workspace-runtime/optimistic/descriptorBuilders';
import { dispatchWorkspaceRuntimeEvent } from '../workspace-runtime/state/workspaceStoreBridge';

const MODS_VIEW_SYNC_TTL_MS = 5_000;
const WINDOW_REFOCUS_MIN_BLUR_MS = 750;
const TOAST_SAMPLE_LIMIT = 2;

export function useWatcherLifecycle(activeGame: GameConfig | null) {
  useEffect(() => {
    let cancelled = false;

    const init = async () => {
      await commands.stopWatcherCmd().catch(() => {});
      if (cancelled) {
        return;
      }

      if (!activeGame?.mod_path || !activeGame?.id) {
        return;
      }

      await commands
        .startWatcherCmd({
          path: activeGame.mod_path,
          gameId: activeGame.id,
        })
        .catch((error: unknown) => {
          console.error('[DiskReconcile] Failed to start watcher:', error);
        });
    };

    void init();

    return () => {
      cancelled = true;
      commands.stopWatcherCmd().catch(() => {});
    };
  }, [activeGame?.id, activeGame?.mod_path]);
}

function totalChangeCount(counts: {
  added: number;
  removed: number;
  renamed: number;
  modified: number;
}): number {
  return counts.added + counts.removed + counts.renamed + counts.modified;
}

function formatToastNames(names: string[]): string {
  if (names.length === 0) {
    return '';
  }

  if (names.length <= TOAST_SAMPLE_LIMIT) {
    return names.join(', ');
  }

  return `${names.slice(0, TOAST_SAMPLE_LIMIT).join(', ')}, +${names.length - TOAST_SAMPLE_LIMIT} others`;
}

function maybeShowExternalChangeToast(result: DiskReconcileResult) {
  if (
    result.reason === 'StartupBoot' ||
    result.reason === 'InternalMutation' ||
    result.reason === 'OnboardingCompleted' ||
    result.reason === 'GameSwitched' ||
    !result.change_summary.has_user_visible_changes
  ) {
    return;
  }

  const objectCount = totalChangeCount(result.change_summary.object_changes);
  const modCount = totalChangeCount(result.change_summary.mod_changes);
  const messages: string[] = [];

  if (objectCount > 0) {
    const names = formatToastNames(result.change_summary.object_sample_names);
    messages.push(
      names
        ? `${objectCount} object folder changes: ${names}`
        : `${objectCount} object folder changes detected`,
    );
  }

  if (modCount > 0) {
    const names = formatToastNames(result.change_summary.mod_sample_names);
    messages.push(
      names ? `${modCount} mod folder changes: ${names}` : `${modCount} mod folder changes detected`,
    );
  }

  if (messages.length > 0) {
    toast.info(messages.join(' | '), 5000);
  }
}

function applyPathUpdates(result: DiskReconcileResult, activeGame: GameConfig | null) {
  const appStore = useAppStore.getState();
  const modsPath = activeGame?.mod_path;
  const rewrites: Array<{ oldPath: string; newPath: string }> = [];

  for (const update of result.path_updates) {
    if (update.kind !== 'Mod' || !modsPath) {
      rewrites.push({
        oldPath: update.from,
        newPath: update.to,
      });
      continue;
    }

    const absoluteFrom = joinModPath(modsPath, update.from);
    const absoluteTo = joinModPath(modsPath, update.to);
    rewrites.push({
      oldPath: absoluteFrom,
      newPath: absoluteTo,
    });
    appStore.replaceGridSelection(absoluteFrom, absoluteTo);
  }

  if (rewrites.length === 0) {
    return;
  }

  dispatchWorkspaceRuntimeEvent({
    type: 'PATHS_REWRITTEN',
    rewrites,
  });
}

function clearStaleSelections(result: DiskReconcileResult, activeGame: GameConfig | null) {
  const appStore = useAppStore.getState();
  const modsPath = activeGame?.mod_path;

  if (result.cleared_selection_paths.length === 0) {
    return;
  }

  const invalidatedPaths = [...result.cleared_selection_paths];
  if (modsPath) {
    invalidatedPaths.push(
      ...result.cleared_selection_paths.map((path) => joinModPath(modsPath, path)),
    );
  }

  dispatchWorkspaceRuntimeEvent({
    type: 'TARGETS_INVALIDATED',
    paths: invalidatedPaths,
    resetExplorer: true,
  });

  if (!modsPath || appStore.gridSelection.size === 0) {
    return;
  }

  const selectedPaths = Array.from(appStore.gridSelection);
  const shouldClearGridSelection = result.cleared_selection_paths.some((path) => {
    const absoluteRoot = joinModPath(modsPath, path);
    return selectedPaths.some((selectedPath) => isSameOrDescendantPath(selectedPath, absoluteRoot));
  });

  if (shouldClearGridSelection) {
    appStore.clearGridSelection();
  }
}

function isPreviewAffected(result: DiskReconcileResult, activeGame: GameConfig | null): boolean {
  if (!activeGame?.mod_path) {
    return false;
  }

  const appStore = useAppStore.getState();
  const selectedPaths = Array.from(appStore.gridSelection);
  const selectedModPath =
    selectedPaths.length > 0 ? selectedPaths[selectedPaths.length - 1] : undefined;
  if (!selectedModPath) {
    return false;
  }

  const selectedPath = normalizePath(selectedModPath);
  const selectedObjectPath = appStore.selectedObjectFolderPath
    ? normalizePath(appStore.selectedObjectFolderPath)
    : null;

  for (const update of result.path_updates) {
    if (update.kind === 'Mod') {
      const absoluteFrom = joinModPath(activeGame.mod_path, update.from);
      const absoluteTo = joinModPath(activeGame.mod_path, update.to);
      if (
        rewritePath(selectedPath, absoluteFrom, absoluteTo) ||
        isSameOrDescendantPath(selectedPath, absoluteTo)
      ) {
        return true;
      }
      continue;
    }

    const objectRewrite = selectedObjectPath ? rewritePath(selectedObjectPath, update.from, update.to) : null;
    if (objectRewrite || (selectedObjectPath && isSameOrDescendantPath(selectedObjectPath, update.to))) {
      return true;
    }
  }

  for (const clearedPath of result.cleared_selection_paths) {
    const absoluteRoot = joinModPath(activeGame.mod_path, clearedPath);
    if (isSameOrDescendantPath(selectedPath, absoluteRoot)) {
      return true;
    }
  }

  for (const changedRoot of result.changed_roots) {
    const absoluteRoot = joinModPath(activeGame.mod_path, changedRoot);
    if (isSameOrDescendantPath(selectedPath, absoluteRoot)) {
      return true;
    }
  }

  for (const thumbnailRoot of result.thumbnail_roots) {
    const absoluteRoot = joinModPath(activeGame.mod_path, thumbnailRoot);
    if (isSameOrDescendantPath(selectedPath, absoluteRoot)) {
      return true;
    }
  }

  return false;
}

export function applyDiskReconcileResult(
  result: DiskReconcileResult,
  queryClient: QueryClient,
  activeGame: GameConfig | null,
) {
  // Disk Reconcile owns filesystem truth and global runtime refresh for disk-backed changes.
  const appStore = useAppStore.getState();
  appStore.setDiskReconcileTimestamp(result.game_id, Date.now());
  applyPathUpdates(result, activeGame);
  clearStaleSelections(result, activeGame);

  const objectListAffected =
    result.objects_changed || result.folders_changed || result.path_updates.length > 0;

  if (objectListAffected) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('objectRows'),
      'active',
    );
  }

  if (result.folders_changed) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('folderStructureOnly'),
      'active',
    );
  }

  if (result.thumbnail_roots.length > 0) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('thumbnailOnly'),
      'active',
    );
  }

  if (result.collections_changed) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor(['collectionsCatalog', 'dashboardKeybindings']),
      'none',
    );
  }

  if (
    result.objects_changed ||
    result.folders_changed ||
    result.runtime_file_changed ||
    result.thumbnail_roots.length > 0
  ) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('dashboardKeybindings'),
      'none',
    );
  }

  if (isPreviewAffected(result, activeGame)) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('previewOnly'),
      'active',
    );
  }

  if (result.folders_changed || result.objects_changed) {
    void publishRuntimeDescriptor(
      queryClient,
      buildRuntimeMutationDescriptor('conflictsOnly'),
      'none',
    );
  }

  maybeShowExternalChangeToast(result);
}

export function useDiskReconcileCoordinator(activeGame: GameConfig | null, queryClient: QueryClient) {
  const workspaceView = useAppStore((state) => state.workspaceView);
  const lastDiskReconcileAtByGame = useAppStore((state) => state.lastDiskReconcileAtByGame);
  const pendingDiskReconcileByGame = useAppStore((state) => state.pendingDiskReconcileByGame);
  const markDiskReconcilePending = useAppStore((state) => state.markDiskReconcilePending);
  const inFlightRef = useRef(false);
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

      const lastSyncAt = lastDiskReconcileAtByGame[gameId] ?? 0;
      const isDirty = pendingDiskReconcileByGame[gameId] ?? false;
      if (isDirty) {
        return true;
      }

      return Date.now() - lastSyncAt > MODS_VIEW_SYNC_TTL_MS;
    },
    [lastDiskReconcileAtByGame, pendingDiskReconcileByGame],
  );

  const runRefresh = useCallback(
    async (reason: DiskReconcileReason, forceFull: boolean) => {
      if (!activeGame?.id || inFlightRef.current) {
        return;
      }

      if (!shouldSync(activeGame.id, forceFull)) {
        return;
      }

      inFlightRef.current = true;
      markDiskReconcilePending(activeGame.id, true);

      try {
        // Disk Reconcile only. This path must never trigger the Deep Match Scanner.
        const result = await commands.reconcileDiskState({
          gameId: activeGame.id,
          reason,
          forceFull,
        });
        applyDiskReconcileResult(result, queryClient, activeGame);
        markGameHydrated(activeGame.id);
      } catch (error) {
        console.error('[DiskReconcile] Refresh failed:', error);
      } finally {
        inFlightRef.current = false;
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

    const unlistenFocusPromise = listen('tauri://focus', () => {
      if (workspaceView !== 'mods') {
        return;
      }

      const gameId = activeGame.id;
      const pending = pendingDiskReconcileByGame[gameId] ?? false;
      const requiresFull = requiresFullReconcileByGameRef.current[gameId] ?? false;
      const isHydrated = hydratedModsViewByGameRef.current[gameId] ?? false;
      const lastBlurAt = lastWindowBlurAtRef.current;
      const blurElapsed = lastBlurAt > 0 ? Date.now() - lastBlurAt : Number.POSITIVE_INFINITY;
      if (
        !pending &&
        !requiresFull &&
        isHydrated &&
        blurElapsed < WINDOW_REFOCUS_MIN_BLUR_MS
      ) {
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
  }, [activeGame?.id, activeGame, pendingDiskReconcileByGame, runRefresh, workspaceView]);
}
