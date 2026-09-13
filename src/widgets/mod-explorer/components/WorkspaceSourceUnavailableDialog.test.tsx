import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { open } from '@tauri-apps/plugin-dialog';
import { commands } from '../../../shared/api/tauri/bindings';
import { useAppStore } from '@/app/store';
import WorkspaceSourceUnavailableDialog from './WorkspaceSourceUnavailableDialog';

const fixtures = vi.hoisted(() => ({
  activeGame: {
    id: 'game-1',
    name: 'GIMI',
    game_type: 0,
    mod_path: 'E:/Missing/Mods',
    game_exe: 'E:/GIMI',
    loader_exe: null,
    launch_args: null,
  },
  settings: {
    games: [] as Array<Record<string, unknown>>,
  },
  saveSettingsAsync: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: fixtures.activeGame }),
}));
vi.mock('@/entities/settings', () => ({
  useSettings: () => ({
    settings: fixtures.settings,
    saveSettingsAsync: fixtures.saveSettingsAsync,
  }),
}));
vi.mock('../../../shared/api/tauri/bindings', () => ({
  commands: {
    checkPathExistsCmd: vi.fn(),
    resolveGameFolder: vi.fn(),
    inspectGameModsDirectory: vi.fn(),
    applyGameModsDirectory: vi.fn(),
    getSettings: vi.fn(),
    reconcileDiskStateCmd: vi.fn(),
  },
}));
vi.mock('@/features/file-watcher/hooks/useFileWatcher', () => ({
  applyDiskReconcileResult: vi.fn(),
}));
vi.mock('@/shared/ui/toast', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));
vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
  useTranslation: () => ({ t: (key: string) => key }),
}));

function renderDialog() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <WorkspaceSourceUnavailableDialog />
    </QueryClientProvider>,
  );
}

describe('WorkspaceSourceUnavailableDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fixtures.settings.games = [{ ...fixtures.activeGame }];
    useAppStore.setState({ diskReconcileByGame: {} });
    vi.mocked(commands.checkPathExistsCmd).mockResolvedValue(false);
    vi.mocked(commands.resolveGameFolder).mockResolvedValue({
      path: 'E:/GIMI',
      mods_path: 'E:/GIMI/Mods',
      launcher_path: 'E:/GIMI/3DMigoto Loader.exe',
    });
    vi.mocked(commands.reconcileDiskStateCmd).mockResolvedValue({} as never);
    vi.mocked(commands.inspectGameModsDirectory).mockResolvedValue({
      game_id: 'game-1',
      candidate_path: 'E:/GIMI/Mods',
      fingerprint: 'candidate-v1',
      summary: {
        classification: 'Matching',
        existing_object_count: 1,
        existing_mod_count: 1,
        candidate_object_count: 1,
        candidate_mod_count: 1,
        physical_entry_count: 1,
        filesystem_identity_match_count: 2,
        relative_path_match_count: 2,
        requires_confirmation: false,
      },
    });
    vi.mocked(commands.applyGameModsDirectory).mockResolvedValue({
      game: { ...fixtures.activeGame, mod_path: 'E:/GIMI/Mods' },
      inspection: {} as never,
      reconcile: {} as never,
    });
    vi.mocked(commands.getSettings).mockResolvedValue({
      revision: 2,
      theme: 'dark',
      language: 'en',
      games: [{ ...fixtures.activeGame, mod_path: 'E:/GIMI/Mods' }],
      active_game_id: 'game-1',
      safety: {
        keywords: [],
      },
      ai: { enabled: false, has_api_key: false, base_url: null },
      auto_close_launcher: false,
    });
    vi.mocked(fixtures.saveSettingsAsync).mockResolvedValue(undefined);
  });

  it('opens on first load when the saved Mods path is missing', async () => {
    renderDialog();

    expect(await screen.findByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('E:/Missing/Mods')).toBeInTheDocument();
  });

  it('inspects and atomically applies a matching relocated Mods path', async () => {
    renderDialog();
    await screen.findByRole('dialog');
    vi.mocked(open).mockResolvedValue('E:/GIMI');

    fireEvent.click(screen.getByRole('button', { name: 'grid:banners.source_locate_btn' }));

    await waitFor(() => expect(commands.resolveGameFolder).toHaveBeenCalledWith('E:/GIMI'));
    expect(commands.inspectGameModsDirectory).toHaveBeenCalledWith('game-1', 'E:/GIMI/Mods');
    expect(commands.applyGameModsDirectory).toHaveBeenCalledWith({
      game_id: 'game-1',
      candidate_path: 'E:/GIMI/Mods',
      expected_fingerprint: 'candidate-v1',
      confirm_empty: false,
      different_confirmation_game_name: null,
    });
    expect(fixtures.saveSettingsAsync).not.toHaveBeenCalled();
  });

  it('requires explicit confirmation before applying an empty folder', async () => {
    vi.mocked(commands.inspectGameModsDirectory).mockResolvedValue({
      game_id: 'game-1',
      candidate_path: 'E:/Empty/Mods',
      fingerprint: 'empty-v1',
      summary: {
        classification: 'Empty',
        existing_object_count: 1,
        existing_mod_count: 1,
        candidate_object_count: 0,
        candidate_mod_count: 0,
        physical_entry_count: 0,
        filesystem_identity_match_count: 0,
        relative_path_match_count: 0,
        requires_confirmation: true,
      },
    });
    vi.mocked(open).mockResolvedValue('E:/Empty');
    renderDialog();
    await screen.findByRole('dialog');

    fireEvent.click(screen.getByRole('button', { name: 'grid:banners.source_locate_btn' }));
    expect(await screen.findByText('grid:banners.source_empty_warning')).toBeInTheDocument();
    expect(commands.applyGameModsDirectory).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'grid:banners.source_confirm_empty_btn' }));
    await waitFor(() =>
      expect(commands.applyGameModsDirectory).toHaveBeenCalledWith(
        expect.objectContaining({ confirm_empty: true }),
      ),
    );
  });

  it('requires the exact game name before applying a different library', async () => {
    vi.mocked(commands.inspectGameModsDirectory).mockResolvedValue({
      game_id: 'game-1',
      candidate_path: 'E:/Other/Mods',
      fingerprint: 'different-v1',
      summary: {
        classification: 'Different',
        existing_object_count: 2,
        existing_mod_count: 4,
        candidate_object_count: 1,
        candidate_mod_count: 2,
        physical_entry_count: 1,
        filesystem_identity_match_count: 0,
        relative_path_match_count: 0,
        requires_confirmation: true,
      },
    });
    vi.mocked(open).mockResolvedValue('E:/Other');
    renderDialog();
    await screen.findByRole('dialog');

    fireEvent.click(screen.getByRole('button', { name: 'grid:banners.source_locate_btn' }));
    const warning = await screen.findByText('grid:banners.source_different_warning');
    expect(warning).toBeInTheDocument();
    const confirmButton = screen.getByRole('button', {
      name: 'grid:banners.source_confirm_different_btn',
    });
    expect(confirmButton).toBeDisabled();

    fireEvent.change(screen.getByRole('textbox'), { target: { value: 'GIMI' } });
    expect(confirmButton).toBeEnabled();
    fireEvent.click(confirmButton);

    await waitFor(() =>
      expect(commands.applyGameModsDirectory).toHaveBeenCalledWith(
        expect.objectContaining({
          different_confirmation_game_name: 'GIMI',
        }),
      ),
    );
  });
});
