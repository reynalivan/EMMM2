import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider, useQueryClient } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import GlobalActions from './GlobalActions';
import { commands, type DiskReconcileResult } from '../../../lib/bindings';
import { applyDiskReconcileResult } from '../../../features/file-watcher/hooks';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { useAppStore } from '../../../stores/useAppStore';
import { useToastStore } from '../../../stores/useToastStore';

vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock('../../../lib/bindings', () => ({
  commands: { openRecycleBin: vi.fn(), reconcileDiskStateCmd: vi.fn() },
}));

vi.mock('../../../features/file-watcher/hooks', () => ({
  applyDiskReconcileResult: vi.fn(),
}));

vi.mock('../../../hooks/useActiveGame', () => ({
  useActiveGame: vi.fn(),
}));

vi.mock('../../../features/launch-bar/LaunchBar', () => ({
  default: () => null,
}));

const activeGame = {
  id: 'game-1',
  name: 'Genshin Impact',
  game_type: 0,
  mod_path: 'E:\\Mods',
  game_exe: 'E:\\Game.exe',
  loader_exe: null,
  launch_args: null,
};

function reconcileResult(status: DiskReconcileResult['status'] = 'Applied'): DiskReconcileResult {
  return {
    game_id: activeGame.id,
    reason: 'ManualRepair',
    status,
    folder_conflicts: [],
    rename_confirmations: [],
    error_message: status === 'SourceUnavailable' ? 'Folder missing' : null,
    changed_roots: [],
    objects_changed: false,
    folders_changed: false,
    collections_changed: false,
    runtime_file_changed: false,
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
    pending_runtime_effects: { collections_dirty: false, overlay_refresh: false },
    warnings: [],
  };
}

function renderActions() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  vi.mocked(useQueryClient).mockReturnValue(queryClient);
  render(
    <QueryClientProvider client={queryClient}>
      <GlobalActions />
    </QueryClientProvider>,
  );
  return queryClient;
}

describe('GlobalActions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.setState({
      workspaceView: 'dashboard',
      isPreviewOpen: false,
      diskReconcileByGame: {},
    });
    useToastStore.setState({ toasts: [] });
    vi.mocked(useActiveGame).mockReturnValue({
      activeGame,
      isLoading: false,
      error: null,
      games: [activeGame],
    });
  });

  it('opens the native Recycle Bin without mounting an app-managed trash dialog', async () => {
    vi.mocked(commands.openRecycleBin).mockResolvedValue(undefined);
    renderActions();

    fireEvent.click(screen.getByTitle('actions.trash'));

    await waitFor(() => expect(commands.openRecycleBin).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('runs one forced full reconcile from the global refresh action', async () => {
    const result = reconcileResult();
    vi.mocked(commands.reconcileDiskStateCmd).mockResolvedValue(result);
    const queryClient = renderActions();

    fireEvent.click(screen.getByTitle('actions.refresh'));

    await waitFor(() =>
      expect(commands.reconcileDiskStateCmd).toHaveBeenCalledWith(
        activeGame.id,
        'ManualRepair',
        null,
        true,
      ),
    );
    await waitFor(() =>
      expect(applyDiskReconcileResult).toHaveBeenCalledWith(result, queryClient, activeGame),
    );
    expect(useToastStore.getState().toasts).toContainEqual(
      expect.objectContaining({ type: 'success', message: 'actions.refresh_already_synced' }),
    );
  });

  it('blocks duplicate refresh clicks while the full reconcile is running', async () => {
    let resolveReconcile!: (result: DiskReconcileResult) => void;
    vi.mocked(commands.reconcileDiskStateCmd).mockReturnValue(
      new Promise((resolve) => {
        resolveReconcile = resolve;
      }),
    );
    renderActions();

    const refreshButton = screen.getByTitle('actions.refresh');
    fireEvent.click(refreshButton);
    fireEvent.click(refreshButton);

    expect(commands.reconcileDiskStateCmd).toHaveBeenCalledTimes(1);
    expect(refreshButton).toBeDisabled();
    expect(useToastStore.getState().toasts).toContainEqual(
      expect.objectContaining({ type: 'info', message: 'actions.refresh_started', duration: 0 }),
    );

    resolveReconcile(reconcileResult());
    await waitFor(() => expect(refreshButton).not.toBeDisabled());
  });

  it('disables global refresh while another reconcile is reporting progress', () => {
    useAppStore.setState({
      diskReconcileByGame: {
        [activeGame.id]: {
          at: 0,
          pending: true,
          unavailable: null,
          progress: {
            game_id: activeGame.id,
            run_id: 'run-1',
            reason: 'WatcherBatch',
            phase: 'ScanningRoots',
            completed_units: 1,
            total_units: 2,
            current_root: 'Alice',
            elapsed_ms: 100,
            eta_ms: 100,
          },
        },
      },
    });

    renderActions();

    expect(screen.getByTitle('actions.refresh')).toBeDisabled();
  });

  it.each([
    ['AppliedWithFolderConflicts', 'warning', 'actions.refresh_conflicts'],
    ['NeedsRenameConfirmation', 'warning', 'actions.refresh_rename_confirmation'],
    ['SourceUnavailable', 'error', 'actions.refresh_source_unavailable'],
  ] as const)('reports %s with the matching terminal toast', async (status, type, message) => {
    vi.mocked(commands.reconcileDiskStateCmd).mockResolvedValue(reconcileResult(status));
    renderActions();

    fireEvent.click(screen.getByTitle('actions.refresh'));

    await waitFor(() =>
      expect(useToastStore.getState().toasts).toContainEqual(
        expect.objectContaining({ type, message }),
      ),
    );
  });
});
