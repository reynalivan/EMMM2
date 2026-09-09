import { act, renderHook, waitFor } from '@testing-library/react';
import { listen } from '@tauri-apps/api/event';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { QueryClient } from '@tanstack/react-query';
import {
  applyDiskReconcileResult,
  useDiskReconcileCoordinator,
  useWatcherLifecycle,
} from './useFileWatcher';
import { isPreviewAffected } from '../utils/reconcileSelection';
import type { DiskReconcileResult } from '../../../shared/api/tauri/bindings';
import { commands } from '../../../shared/api/tauri/bindings';
import { runtimeQueryKeys } from '@/shared/lib/queryRefresh';
import { GameType, type GameConfig } from '@/entities/game';
import { useAppStore } from '@/app/store';
import { workspaceKeys } from '@/features/workspace-runtime/@x/file-watcher';

vi.mock('../../../shared/api/tauri/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    stopWatcher: vi.fn().mockResolvedValue(undefined),
    startWatcher: vi.fn().mockResolvedValue(undefined),
    reconcileDiskStateCmd: vi.fn(),
  },
}));

vi.mock('@/app/store', () => {
  const state = {
    workspaceView: 'mods',
    explorerSubPath: undefined as string | undefined,
    selectedObjectFolderPath: null as string | null,
    selectedModPath: null as string | null,
    gridSelection: new Set<string>(),
    diskReconcileByGame: {} as Record<
      string,
      { at: number; pending: boolean; unavailable: string | null }
    >,
    folderConflictsByGame: {},
    renameConfirmationsByGame: {},
    setDiskReconcileTimestamp: vi.fn(),
    setDiskReconcileProgress: vi.fn(),
    markDiskReconcilePending: vi.fn(),
    setDiskSourceUnavailable: vi.fn(),
    setFolderConflicts: vi.fn(),
    setRenameConfirmations: vi.fn(),
    setExplorerSubPath: vi.fn(),
    setCurrentPath: vi.fn(),
    setSelectedObjectFolderPath: vi.fn(),
    replaceGridSelections: vi.fn(),
    clearGridSelection: vi.fn(),
    // Workspace runtime slice: the bridge dispatches straight into the store.
    currentPath: [] as string[],
    mobileActivePane: 'sidebar' as const,
    workspacePreviewDirty: false,
    workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
    workspaceDialogState: { kind: 'none' },
    dispatchWorkspaceRuntime: vi.fn(),
  };

  const useAppStore = Object.assign(
    vi.fn((selector?: (value: typeof state) => unknown) => (selector ? selector(state) : state)),
    {
      getState: vi.fn(() => state),
    },
  );

  return { useAppStore };
});

type MockEventHandler = (event: { payload: unknown }) => void;

function createActiveGame(): GameConfig {
  return {
    id: 'game-1',
    mod_path: 'E:/Mods',
    game_type: GameType.GIMI,
    name: 'Genshin',
    game_exe: 'game.exe',
    loader_exe: null,
    launch_args: null,
  };
}

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });

  return { promise, resolve, reject };
}

vi.mock('@/shared/ui/toast', () => ({
  toast: {
    info: vi.fn(),
    warning: vi.fn(),
  },
}));

function createResult(overrides: Partial<DiskReconcileResult>): DiskReconcileResult {
  return {
    game_id: 'game-1',
    reason: 'WatcherBatch',
    changed_roots: [],
    objects_changed: false,
    folders_changed: false,
    collections_changed: false,
    runtime_file_changed: false,
    status: 'Applied',
    folder_conflicts: [],
    rename_confirmations: [],
    error_message: null,
    thumbnail_roots: [],
    cleared_selection_paths: [],
    path_updates: [],
    pending_runtime_effects: {
      collections_dirty: false,
      overlay_refresh: false,
    },
    warnings: [],
    collection_reference_impact: {
      affected_collection_count: 0,
      affected_collection_names: [],
      rewritten_paths: [],
      missing_paths: [],
    },
    change_summary: {
      object_changes: { added: 0, removed: 0, renamed: 0, modified: 0 },
      mod_changes: { added: 0, removed: 0, renamed: 0, modified: 0 },
      object_sample_names: [],
      mod_sample_names: [],
      has_user_visible_changes: false,
    },
    ...overrides,
  };
}

describe('useWatcherLifecycle', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('starts through the backend atomic replacement without waiting for a frontend stop', async () => {
    const pendingStop = createDeferred<void>();
    vi.mocked(commands.stopWatcher).mockReturnValueOnce(pendingStop.promise);

    const { unmount } = renderHook(() => useWatcherLifecycle(createActiveGame()));

    await waitFor(() => expect(commands.startWatcher).toHaveBeenCalledWith('E:/Mods', 'game-1'));
    expect(commands.stopWatcher).not.toHaveBeenCalled();
    unmount();
    pendingStop.resolve();
    await waitFor(() => expect(commands.stopWatcher).toHaveBeenCalledTimes(1));
  });

  it('does not let stale cleanup stop a newly selected game watcher', async () => {
    const gameA = createActiveGame();
    const gameB = { ...createActiveGame(), id: 'game-2', mod_path: 'F:/Mods' };
    const { rerender, unmount } = renderHook(
      ({ game }: { game: GameConfig | null }) => useWatcherLifecycle(game),
      { initialProps: { game: gameA } },
    );

    await waitFor(() => expect(commands.startWatcher).toHaveBeenCalledWith('E:/Mods', 'game-1'));
    rerender({ game: gameB });
    await waitFor(() => expect(commands.startWatcher).toHaveBeenCalledWith('F:/Mods', 'game-2'));
    await act(async () => {
      await Promise.resolve();
    });

    expect(commands.stopWatcher).not.toHaveBeenCalled();
    unmount();
    await waitFor(() => expect(commands.stopWatcher).toHaveBeenCalledTimes(1));
  });
});

describe('applyDiskReconcileResult', () => {
  const queryClient = {
    invalidateQueries: vi.fn(),
    setQueriesData: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    const state = useAppStore.getState();
    state.workspaceView = 'mods';
    state.selectedObjectFolderPath = null;
    state.selectedModPath = null;
    state.gridSelection = new Set();
    state.diskReconcileByGame = {};
  });

  it('refreshes ObjectList when folders change', async () => {
    applyDiskReconcileResult(
      createResult({ folders_changed: true }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      {
        id: 'game-1',
        mod_path: 'E:/Mods',
        game_type: GameType.GIMI,
        name: 'Genshin',
        game_exe: 'game.exe',
        loader_exe: null,
        launch_args: null,
      },
    );
    await Promise.resolve();

    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.workspaceViewModel,
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.objectRows,
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.objectCounts,
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.folderStructure,
      refetchType: 'active',
    });
  });

  it('refreshes the shallow workspace after a no-op initial recovery', () => {
    applyDiskReconcileResult(
      createResult({ reason: 'StartupBoot' }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      createActiveGame(),
    );

    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: workspaceKeys.all,
      refetchType: 'active',
    });
  });

  it('auto-opens an applied folder-conflict report once per fingerprint', () => {
    const state = useAppStore.getState();
    const group = {
      group_id: 'group-1',
      identity: 'alice/blue',
      display_name: 'Blue',
      candidates: [
        { path: 'E:/Mods/Alice/Blue', folder_name: 'Blue', base_name: 'Blue', is_enabled: true },
        {
          path: 'E:/Mods/Alice/DISABLED Blue',
          folder_name: 'DISABLED Blue',
          base_name: 'Blue',
          is_enabled: false,
        },
      ],
    };

    applyDiskReconcileResult(
      createResult({ status: 'AppliedWithFolderConflicts', folder_conflicts: [group] }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      createActiveGame(),
    );

    // Re-delivering the same watcher report keeps the banner current without
    // stealing focus by reopening a dialog the user already dismissed.
    applyDiskReconcileResult(
      createResult({ status: 'AppliedWithFolderConflicts', folder_conflicts: [group] }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      createActiveGame(),
    );

    const changedGroup = {
      ...group,
      candidates: group.candidates.map((candidate, index) =>
        index === 0 ? { ...candidate, is_enabled: false } : candidate,
      ),
    };
    applyDiskReconcileResult(
      createResult({ status: 'AppliedWithFolderConflicts', folder_conflicts: [changedGroup] }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      createActiveGame(),
    );

    expect(state.setFolderConflicts).toHaveBeenCalledWith('game-1', [group]);
    expect(state.dispatchWorkspaceRuntime).toHaveBeenCalledTimes(2);
    expect(state.dispatchWorkspaceRuntime).toHaveBeenCalledWith({
      type: 'DIALOG_OPENED',
      dialog: { kind: 'folderConflicts' },
    });
    expect(state.setDiskReconcileTimestamp).toHaveBeenCalled();
  });

  it('refreshes safe workspace data while retaining an applied conflict report', async () => {
    const state = useAppStore.getState();
    const group = {
      group_id: 'group-safe-scope',
      identity: 'alice/blue',
      display_name: 'Blue',
      candidates: [
        { path: 'E:/Mods/Alice/Blue', folder_name: 'Blue', base_name: 'Blue', is_enabled: true },
        {
          path: 'E:/Mods/Alice/DISABLED Blue',
          folder_name: 'DISABLED Blue',
          base_name: 'Blue',
          is_enabled: false,
        },
      ],
    };

    applyDiskReconcileResult(
      createResult({
        status: 'AppliedWithFolderConflicts',
        folder_conflicts: [group],
        folders_changed: true,
      }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      createActiveGame(),
    );
    await Promise.resolve();

    expect(state.setFolderConflicts).toHaveBeenCalledWith('game-1', [group]);
    expect(state.setDiskReconcileTimestamp).toHaveBeenCalled();
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.folderStructure,
      refetchType: 'active',
    });
  });

  it('stores and auto-opens each new rename-confirmation report once', () => {
    const state = useAppStore.getState();
    const group = {
      group_id: 'rename-group-1',
      kind: 'Mod' as const,
      reason: 'MissingIdentity' as const,
      scope_key: 'alice',
      previous_paths: ['Alice/Old'],
      current_paths: ['Alice/New'],
      previous_path_count: 1,
      current_path_count: 1,
      candidates_truncated: false,
    };
    const result = createResult({
      status: 'NeedsRenameConfirmation',
      rename_confirmations: [group],
    });

    applyDiskReconcileResult(
      result,
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      createActiveGame(),
    );
    applyDiskReconcileResult(
      result,
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      createActiveGame(),
    );

    expect(state.setRenameConfirmations).toHaveBeenCalledWith('game-1', [group]);
    expect(state.dispatchWorkspaceRuntime).toHaveBeenCalledTimes(1);
    expect(state.dispatchWorkspaceRuntime).toHaveBeenCalledWith({
      type: 'DIALOG_OPENED',
      dialog: { kind: 'renameConfirmations' },
    });
    expect(state.setDiskReconcileTimestamp).not.toHaveBeenCalled();
    expect(queryClient.invalidateQueries).not.toHaveBeenCalled();
  });

  it('refreshes ObjectList when path updates rewrite object-relative paths', async () => {
    applyDiskReconcileResult(
      createResult({
        path_updates: [{ from: 'Old/Object', to: 'New/Object', kind: 'Object' }],
      }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      {
        id: 'game-1',
        mod_path: 'E:/Mods',
        game_type: GameType.GIMI,
        name: 'Genshin',
        game_exe: 'game.exe',
        loader_exe: null,
        launch_args: null,
      },
    );
    await Promise.resolve();

    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.workspaceViewModel,
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.objectRows,
      refetchType: 'active',
    });
  });

  it('rewrites store selection paths before publishing reconcile refresh', async () => {
    const queryClientWithCache = new QueryClient();
    const oldPath = 'E:/Mods/ALBEDO/Variant';
    const newPath = 'E:/Mods/ALBEDO/Variant Renamed';

    applyDiskReconcileResult(
      createResult({
        path_updates: [{ from: 'ALBEDO/Variant', to: 'ALBEDO/Variant Renamed', kind: 'Mod' }],
      }),
      queryClientWithCache,
      {
        id: 'game-1',
        mod_path: 'E:/Mods',
        game_type: GameType.GIMI,
        name: 'Genshin',
        game_exe: 'game.exe',
        loader_exe: null,
        launch_args: null,
      },
    );

    // Invalidation-only: the cached view model is refetched, not patched; the
    // store selection is what must follow the rename synchronously.
    expect(useAppStore.getState().replaceGridSelections).toHaveBeenCalledWith([
      { oldPath, newPath },
    ]);
  });

  it('invalidates thumbnail queries when watcher reports thumbnail roots', async () => {
    applyDiskReconcileResult(
      createResult({
        thumbnail_roots: ['Albedo'],
      }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      {
        id: 'game-1',
        mod_path: 'E:/Mods',
        game_type: GameType.GIMI,
        name: 'Genshin',
        game_exe: 'game.exe',
        loader_exe: null,
        launch_args: null,
      },
    );
    await Promise.resolve();

    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.thumbnails,
      refetchType: 'active',
    });
  });

  it('refreshes metadata and preview queries when a runtime file changes on disk', async () => {
    const state = useAppStore.getState();
    state.selectedModPath = 'E:/Mods/ALBEDO/Variant';

    applyDiskReconcileResult(
      createResult({
        changed_roots: ['ALBEDO'],
        runtime_file_changed: true,
      }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      createActiveGame(),
    );
    await Promise.resolve();

    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.workspaceViewModel,
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.folderMetadata,
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.previewDetails,
      refetchType: 'active',
    });
  });

  it('records unavailable disk source without refreshing runtime queries', async () => {
    const { useAppStore } = await import('@/app/store');
    const state = useAppStore.getState();

    applyDiskReconcileResult(
      createResult({
        status: 'SourceUnavailable',
        error_message: 'Disk Reconcile mods path is unavailable: E:/Missing',
      }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      {
        id: 'game-1',
        mod_path: 'E:/Missing',
        game_type: GameType.GIMI,
        name: 'Genshin',
        game_exe: 'game.exe',
        loader_exe: null,
        launch_args: null,
      },
    );
    await Promise.resolve();

    expect(state.setDiskSourceUnavailable).toHaveBeenCalledWith(
      'game-1',
      'Disk Reconcile mods path is unavailable: E:/Missing',
    );
    expect(queryClient.invalidateQueries).not.toHaveBeenCalled();
  });

  it('clears unavailable disk source after a successful applied result', async () => {
    const { useAppStore } = await import('@/app/store');
    const state = useAppStore.getState();

    applyDiskReconcileResult(
      createResult({ objects_changed: true }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      {
        id: 'game-1',
        mod_path: 'E:/Mods',
        game_type: GameType.GIMI,
        name: 'Genshin',
        game_exe: 'game.exe',
        loader_exe: null,
        launch_args: null,
      },
    );
    await Promise.resolve();

    expect(state.setDiskSourceUnavailable).toHaveBeenCalledWith('game-1', null);
  });

  it('uses active refresh for collections and dashboard scopes changed by reconcile', async () => {
    applyDiskReconcileResult(
      createResult({
        collections_changed: true,
        runtime_file_changed: true,
      }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      {
        id: 'game-1',
        mod_path: 'E:/Mods',
        game_type: GameType.GIMI,
        name: 'Genshin',
        game_exe: 'game.exe',
        loader_exe: null,
        launch_args: null,
      },
    );
    await Promise.resolve();

    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.collections,
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.dashboard,
      refetchType: 'active',
    });
    expect(queryClient.invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.activeKeybindings,
      refetchType: 'active',
    });
  });

  it('includes collection reference impact in the external change toast', async () => {
    const { toast } = await import('@/shared/ui/toast');

    applyDiskReconcileResult(
      createResult({
        collections_changed: true,
        collection_reference_impact: {
          affected_collection_count: 1,
          affected_collection_names: ['Preset A'],
          rewritten_paths: [{ from: 'AINOZ/Old', to: 'AINOZ/New' }],
          missing_paths: [],
        },
        change_summary: {
          object_changes: { added: 0, removed: 0, renamed: 0, modified: 0 },
          mod_changes: { added: 0, removed: 0, renamed: 1, modified: 0 },
          object_sample_names: [],
          mod_sample_names: ['New'],
          has_user_visible_changes: true,
        },
      }),
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      {
        id: 'game-1',
        mod_path: 'E:/Mods',
        game_type: GameType.GIMI,
        name: 'Genshin',
        game_exe: 'game.exe',
        loader_exe: null,
        launch_args: null,
      },
    );

    expect(toast.info).toHaveBeenCalledWith(
      expect.stringContaining('Updated references in 1 collection: Preset A'),
      5000,
    );
  });

  it('surfaces a nonfatal warning when committed runtime effects remain pending', async () => {
    const { toast } = await import('@/shared/ui/toast');

    const result = createResult({
      pending_runtime_effects: {
        collections_dirty: true,
        overlay_refresh: false,
      },
      warnings: [
        {
          kind: 'RuntimeEffectsPending',
          message: 'Collection runtime refresh will be retried',
        },
      ],
    });

    applyDiskReconcileResult(
      result,
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      createActiveGame(),
    );
    applyDiskReconcileResult(
      result,
      queryClient as unknown as import('@tanstack/react-query').QueryClient,
      createActiveGame(),
    );

    expect(toast.warning).toHaveBeenCalledWith(
      'Disk changes were applied, but runtime refresh is still pending.',
    );
    expect(toast.warning).toHaveBeenCalledTimes(1);
  });
});

describe('isPreviewAffected', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const state = useAppStore.getState();
    state.selectedObjectFolderPath = null;
    state.selectedModPath = null;
    state.gridSelection = new Set();
  });

  it('uses selectedModPath when grid selection is empty', () => {
    const state = useAppStore.getState();
    state.selectedModPath = 'E:/Mods/ALBEDO/Variant';

    expect(
      isPreviewAffected(
        createResult({
          changed_roots: ['ALBEDO'],
        }),
        createActiveGame(),
      ),
    ).toBe(true);
  });
});

describe('useDiskReconcileCoordinator', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const state = useAppStore.getState();
    state.workspaceView = 'mods';
    state.diskReconcileByGame = {};
  });

  it('runs a queued focus refresh after the current reconcile completes', async () => {
    const eventHandlers: Record<string, MockEventHandler> = {};
    (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (event: string, callback: MockEventHandler) => {
        eventHandlers[event] = callback;
        return Promise.resolve(vi.fn());
      },
    );
    const firstRefresh = createDeferred<DiskReconcileResult>();
    const reconcileDiskState = commands.reconcileDiskStateCmd as unknown as ReturnType<
      typeof vi.fn
    >;
    reconcileDiskState
      .mockReturnValueOnce(firstRefresh.promise)
      .mockResolvedValueOnce(createResult({ reason: 'WindowRefocused' }));

    renderHook(() => useDiskReconcileCoordinator(createActiveGame(), new QueryClient()));

    await waitFor(() => expect(reconcileDiskState).toHaveBeenCalledTimes(1));

    await act(async () => {
      eventHandlers['tauri://focus']({ payload: null });
    });

    expect(reconcileDiskState).toHaveBeenCalledTimes(1);

    await act(async () => {
      firstRefresh.resolve(createResult({ reason: 'ModsViewEntered' }));
      await firstRefresh.promise;
    });

    await waitFor(() => expect(reconcileDiskState).toHaveBeenCalledTimes(2));
    expect(reconcileDiskState).toHaveBeenLastCalledWith('game-1', 'WindowRefocused', null, false);
  });

  it('keeps a rename-confirmation game unhydrated and forces the next repair to be full', async () => {
    const eventHandlers: Record<string, MockEventHandler> = {};
    (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (event: string, callback: MockEventHandler) => {
        eventHandlers[event] = callback;
        return Promise.resolve(vi.fn());
      },
    );
    const reconcileDiskState = commands.reconcileDiskStateCmd as unknown as ReturnType<
      typeof vi.fn
    >;
    reconcileDiskState
      .mockResolvedValueOnce(
        createResult({ status: 'NeedsRenameConfirmation', rename_confirmations: [] }),
      )
      .mockResolvedValueOnce(createResult({ reason: 'WindowRefocused' }));

    renderHook(() => useDiskReconcileCoordinator(createActiveGame(), new QueryClient()));
    await waitFor(() => expect(reconcileDiskState).toHaveBeenCalledTimes(1));

    await act(async () => {
      eventHandlers['tauri://focus']({ payload: null });
    });

    await waitFor(() => expect(reconcileDiskState).toHaveBeenCalledTimes(2));
    expect(reconcileDiskState).toHaveBeenLastCalledWith('game-1', 'WindowRefocused', null, true);
  });

  it('clears failed reconcile progress and marks the next refresh as a full repair', async () => {
    const eventHandlers: Record<string, MockEventHandler> = {};
    (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (event: string, callback: MockEventHandler) => {
        eventHandlers[event] = callback;
        return Promise.resolve(vi.fn());
      },
    );
    const reconcileDiskState = commands.reconcileDiskStateCmd as unknown as ReturnType<
      typeof vi.fn
    >;
    reconcileDiskState
      .mockRejectedValueOnce(new Error('database temporarily unavailable'))
      .mockResolvedValueOnce(createResult({ reason: 'WindowRefocused' }));

    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    const { toast } = await import('@/shared/ui/toast');
    const state = useAppStore.getState();
    renderHook(() => useDiskReconcileCoordinator(createActiveGame(), new QueryClient()));

    await waitFor(() =>
      expect(state.setDiskReconcileProgress).toHaveBeenCalledWith('game-1', null),
    );
    expect(state.markDiskReconcilePending).toHaveBeenLastCalledWith('game-1', true);
    expect(toast.warning).toHaveBeenCalledTimes(1);

    act(() => {
      eventHandlers['tauri://focus']({ payload: null });
    });

    await waitFor(() => expect(reconcileDiskState).toHaveBeenCalledTimes(2));
    expect(reconcileDiskState).toHaveBeenLastCalledWith('game-1', 'WindowRefocused', null, true);
    consoleError.mockRestore();
  });

  it('deduplicates identical watcher errors for the same game and path for three seconds', async () => {
    const eventHandlers: Record<string, MockEventHandler> = {};
    (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (event: string, callback: MockEventHandler) => {
        eventHandlers[event] = callback;
        return Promise.resolve(vi.fn());
      },
    );
    (commands.reconcileDiskStateCmd as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(
      createResult({}),
    );
    const now = vi.spyOn(Date, 'now').mockReturnValue(1_000);
    const { toast } = await import('@/shared/ui/toast');

    renderHook(() => useDiskReconcileCoordinator(createActiveGame(), new QueryClient()));
    await waitFor(() => expect(eventHandlers['mod_watch:event']).toBeDefined());

    act(() => {
      eventHandlers['mod_watch:event']({
        payload: { type: 'Error', game_id: 'game-1', path: 'E:/Mods', error: 'overflow' },
      });
      eventHandlers['mod_watch:event']({
        payload: { type: 'Error', game_id: 'game-1', path: 'E:/Mods', error: 'overflow' },
      });
    });
    expect(toast.warning).toHaveBeenCalledTimes(1);

    now.mockReturnValue(4_001);
    act(() => {
      eventHandlers['mod_watch:event']({
        payload: { type: 'Error', game_id: 'game-1', path: 'E:/Mods', error: 'overflow' },
      });
    });
    expect(toast.warning).toHaveBeenCalledTimes(2);
    now.mockRestore();
  });
});
