import { act, render, waitFor } from '@testing-library/react';
import { listen } from '@tauri-apps/api/event';
import { QueryClient, QueryClientProvider, useQueryClient } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { commands, type DiskReconcileResult } from '../../lib/bindings';
import { useAppStore } from '../../stores/useAppStore';
import { GameType, type GameConfig } from '../../types/game';
import { runtimeQueryKeys } from '../runtime-sync/queryRefresh';
import { ExternalChangeHandler } from './ExternalChangeHandler';

const activeGameFixture = vi.hoisted((): { current: GameConfig } => ({
  current: {
    id: 'game-1',
    mod_path: 'E:/Mods',
    game_type: 0,
    name: 'Genshin',
    game_exe: 'game.exe',
    loader_exe: null,
    launch_args: null,
  },
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

vi.mock('../../lib/bindings', () => ({
  sparse: (value: unknown) => value,
  commands: {
    stopWatcher: vi.fn().mockResolvedValue(undefined),
    startWatcher: vi.fn().mockResolvedValue(undefined),
    reconcileDiskStateCmd: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({
    activeGame: activeGameFixture.current,
  }),
}));

type EventHandler = (event: { payload: DiskReconcileResult }) => void;

function createResult(overrides: Partial<DiskReconcileResult>): DiskReconcileResult {
  return {
    game_id: 'game-1',
    reason: 'WatcherBatch',
    status: 'Applied',
    error_message: null,
    changed_roots: [],
    objects_changed: false,
    folders_changed: false,
    collections_changed: false,
    runtime_file_changed: false,
    overlay_refresh_triggered: false,
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

function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });
}

describe('ExternalChangeHandler integration', () => {
  let eventHandlers: Record<string, EventHandler>;

  beforeEach(() => {
    vi.clearAllMocks();
    eventHandlers = {};
    activeGameFixture.current = {
      id: 'game-1',
      mod_path: 'E:/Mods',
      game_type: GameType.GIMI,
      name: 'Genshin',
      game_exe: 'game.exe',
      loader_exe: null,
      launch_args: null,
    } satisfies GameConfig;
    useAppStore.setState({
      workspaceView: 'mods',
      selectedObjectFolderPath: 'ALBEDO',
      selectedModPath: 'E:/Mods/ALBEDO/Variant',
      gridSelection: new Set(),
      lastDiskReconcileAtByGame: {},
      pendingDiskReconcileByGame: {},
      diskSourceUnavailableByGame: {},
    });
    (listen as unknown as ReturnType<typeof vi.fn>).mockImplementation(
      (event: string, callback: EventHandler) => {
        eventHandlers[event] = callback;
        return Promise.resolve(vi.fn());
      },
    );
    (commands.reconcileDiskStateCmd as unknown as ReturnType<typeof vi.fn>).mockResolvedValue(
      createResult({ reason: 'ModsViewEntered' }),
    );
  });

  it('refreshes left object list, right folder grid, and preview from a watcher reconcile event', async () => {
    const queryClient = createQueryClient();
    const invalidateQueries = vi.spyOn(queryClient, 'invalidateQueries');
    (useQueryClient as unknown as ReturnType<typeof vi.fn>).mockReturnValue(queryClient);

    render(
      <QueryClientProvider client={queryClient}>
        <ExternalChangeHandler />
      </QueryClientProvider>,
    );

    await waitFor(() => expect(commands.startWatcher).toHaveBeenCalledWith('E:/Mods', 'game-1'));

    await act(async () => {
      eventHandlers['disk_reconcile:result']({
        payload: createResult({
          changed_roots: ['ALBEDO'],
          objects_changed: true,
          folders_changed: true,
          runtime_file_changed: true,
        }),
      });
      await Promise.resolve();
    });

    await waitFor(() =>
      expect(invalidateQueries).toHaveBeenCalledWith({
        queryKey: runtimeQueryKeys.workspaceViewModel,
        refetchType: 'active',
      }),
    );
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.objectRows,
      refetchType: 'active',
    });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.folderStructure,
      refetchType: 'active',
    });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.folderMetadata,
      refetchType: 'active',
    });
    expect(invalidateQueries).toHaveBeenCalledWith({
      queryKey: runtimeQueryKeys.previewDetails,
      refetchType: 'active',
    });
  });
});
