import { act, renderHook, waitFor } from '@testing-library/react';
import { listen } from '@tauri-apps/api/event';
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { QueryClient } from '@tanstack/react-query';
import { isModFolder } from './pathUtils';
import { applyDiskReconcileResult, useDiskReconcileCoordinator } from './hooks';
import { isPreviewAffected } from './reconcileSelection';
import type { DiskReconcileResult } from '../../lib/bindings';
import { commands } from '../../lib/bindings';
import { runtimeQueryKeys } from '../runtime-sync/queryRefresh';
import { GameType, type GameConfig } from '../../types/game';
import { workspaceKeys } from '../workspace-runtime/useWorkspaceViewModel';
import type { WorkspaceViewModel } from '../../types/workspace';
import { useAppStore } from '../../stores/useAppStore';

vi.mock('../../lib/bindings', () => ({
  commands: {
    stopWatcher: vi.fn().mockResolvedValue(undefined),
    startWatcher: vi.fn().mockResolvedValue(undefined),
    reconcileDiskState: vi.fn(),
  },
}));

vi.mock('../../stores/useAppStore', () => {
  const state = {
    workspaceView: 'mods',
    explorerSubPath: undefined as string | undefined,
    selectedObjectFolderPath: null as string | null,
    selectedModPath: null as string | null,
    gridSelection: new Set<string>(),
    lastDiskReconcileAtByGame: {} as Record<string, number>,
    pendingDiskReconcileByGame: {} as Record<string, boolean>,
    diskSourceUnavailableByGame: {} as Record<string, string | null>,
    setDiskReconcileTimestamp: vi.fn(),
    markDiskReconcilePending: vi.fn(),
    setDiskSourceUnavailable: vi.fn(),
    setExplorerSubPath: vi.fn(),
    setCurrentPath: vi.fn(),
    setSelectedObjectFolderPath: vi.fn(),
    replaceGridSelection: vi.fn(),
    replaceGridSelections: vi.fn(),
    clearGridSelection: vi.fn(),
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

vi.mock('../../stores/useToastStore', () => ({
  toast: {
    info: vi.fn(),
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
    overlay_refresh_triggered: false,
    status: 'Applied',
    error_message: null,
    thumbnail_roots: [],
    cleared_selection_paths: [],
    path_updates: [],
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

function createWorkspaceViewModel(selectedPath: string): WorkspaceViewModel {
  return {
    objects: [],
    explorer: {
      self_node_type: null,
      self_node_kind: 'container',
      self_display_mode: 'container_folder',
      self_type_chip: null,
      self_is_mod: false,
      self_is_enabled: true,
      self_is_effectively_active: true,
      self_owner_object_id: null,
      self_owner_object_folder_path: null,
      self_classification_reasons: [],
      children: [
        {
          node_type: 'FlatModRoot',
          classification_reasons: [],
          id: 'mod-1',
          owner_object_id: 'object-1',
          owner_object_folder_path: 'ALBEDO',
          name: 'Variant',
          folder_name: 'Variant',
          path: selectedPath,
          is_enabled: true,
          is_directory: true,
          thumbnail_path: null,
          modified_at: 0,
          size_bytes: 0,
          has_info_json: false,
          is_favorite: false,
          is_misplaced: false,
          is_safe: true,
          metadata: null,
          category: null,
          conflict_group_id: null,
          conflict_state: null,
          warnings: [],
          node_kind: 'terminal_mod',
          display_mode: 'flat_mod',
          type_chip: 'flat_mod',
          display_name: 'Variant',
          is_effectively_active: true,
          ancestor_disabled: false,
          inactive_reason: null,
          warning_state: 'none',
          primary_warning: null,
          switch_state: 'enabled',
          switch_reason: null,
          switch_policy_key: 'mod',
          capabilities: {
            can_toggle: true,
            can_rename: true,
            can_delete: true,
            can_move: true,
            can_toggle_safe: true,
            can_sync: true,
            can_enable_only_this: true,
            can_pin: true,
            can_edit_metadata: true,
            can_reveal_in_explorer: true,
            can_move_category: true,
            can_open_in_explorer: true,
          },
          can_navigate: false,
        },
      ],
      conflicts: [],
      ancestor_disabled_by: null,
      ancestor_disabled_path: null,
      inactive_reason: null,
    },
    preview: {
      selected_path: selectedPath,
      selected_node: null,
      is_flat_mod_root: true,
      display_title: 'Variant',
      display_subtitle: null,
      mod_info_summary: null,
      ini_summary: null,
      image_summary: null,
      warning_summary: {
        state: 'none',
        messages: [],
      },
    },
    selection: {
      selected_object_folder_path: 'ALBEDO',
      explorer_sub_path: 'ALBEDO',
      selected_mod_path: selectedPath,
      current_path: ['ALBEDO'],
      reconciliation_status: 'unchanged',
      reconciliation_reason: null,
      affected_paths: [],
    },
    runtime: {
      game_id: 'game-1',
      safe_mode: false,
      source_state: {
        status: 'available',
        message: null,
      },
    },
  };
}

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
    state.lastDiskReconcileAtByGame = {};
    state.pendingDiskReconcileByGame = {};
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

  it('rewrites cached WorkspaceViewModel paths before publishing reconcile refresh', async () => {
    const queryClientWithCache = new QueryClient();
    const oldPath = 'E:/Mods/ALBEDO/Variant';
    const newPath = 'E:/Mods/ALBEDO/Variant Renamed';
    const workspaceKey = workspaceKeys.viewModel(
      {
        game_id: 'game-1',
        safe_mode: false,
        object_type: null,
        search_query: null,
        meta_filters: null,
        sort_by: null,
        status_filter: null,
      },
      'ALBEDO',
      'ALBEDO',
      oldPath,
    );
    queryClientWithCache.setQueryData(workspaceKey, createWorkspaceViewModel(oldPath));

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

    const workspace = queryClientWithCache.getQueryData<WorkspaceViewModel>(workspaceKey);
    expect(workspace?.selection.selected_mod_path).toBe(newPath);
    expect(workspace?.preview.selected_path).toBe(newPath);
    expect(workspace?.explorer.children[0]?.path).toBe(newPath);
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
    const { useAppStore } = await import('../../stores/useAppStore');
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
    const { useAppStore } = await import('../../stores/useAppStore');
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
    const { toast } = await import('../../stores/useToastStore');

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
    state.lastDiskReconcileAtByGame = {};
    state.pendingDiskReconcileByGame = {};
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
    const reconcileDiskState = commands.reconcileDiskState as unknown as ReturnType<typeof vi.fn>;
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
    expect(reconcileDiskState).toHaveBeenLastCalledWith({
      gameId: 'game-1',
      reason: 'WindowRefocused',
      forceFull: false,
    });
  });
});

describe('isModFolder', () => {
  it('accepts unicode mod folder path with slash and ASCII case variants', () => {
    expect(isModFolder('e:\\mods\\한국character\\日本語mod', 'E:/Mods')).toBe(true);
  });

  it('rejects unicode file path under mod root', () => {
    expect(isModFolder('e:\\mods\\한국character\\日本語mod\\config.ini', 'E:/Mods')).toBe(false);
  });
});
