import { describe, expect, it, vi, beforeEach } from 'vitest';
import { isModFolder } from './pathUtils';
import { applyDiskReconcileResult } from './hooks';
import type { DiskReconcileResult } from '../../lib/bindings';
import { runtimeQueryKeys } from '../runtime-sync/queryRefresh';
import { GameType } from '../../types/game';

vi.mock('../../stores/useAppStore', () => {
  const state = {
    explorerSubPath: undefined as string | undefined,
    selectedObjectFolderPath: null as string | null,
    gridSelection: new Set<string>(),
    setDiskReconcileTimestamp: vi.fn(),
    setDiskSourceUnavailable: vi.fn(),
    setExplorerSubPath: vi.fn(),
    setCurrentPath: vi.fn(),
    setSelectedObjectFolderPath: vi.fn(),
    replaceGridSelection: vi.fn(),
    clearGridSelection: vi.fn(),
  };

  const useAppStore = Object.assign(vi.fn(() => null), {
    getState: vi.fn(() => state),
  });

  return { useAppStore };
});

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

describe('applyDiskReconcileResult', () => {
  const queryClient = {
    invalidateQueries: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
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
});

describe('isModFolder', () => {
  it('accepts unicode mod folder path with slash and ASCII case variants', () => {
    expect(isModFolder('e:\\mods\\한국character\\日本語mod', 'E:/Mods')).toBe(true);
  });

  it('rejects unicode file path under mod root', () => {
    expect(isModFolder('e:\\mods\\한국character\\日本語mod\\config.ini', 'E:/Mods')).toBe(false);
  });
});
