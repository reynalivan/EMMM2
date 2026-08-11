import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { open } from '@tauri-apps/plugin-dialog';
import { commands } from '../../../lib/bindings';
import { useAppStore } from '../../../stores/useAppStore';
import WorkspaceSourceUnavailableDialog from './WorkspaceSourceUnavailableDialog';

const fixtures = vi.hoisted(() => ({
  activeGame: {
    id: 'game-1',
    name: 'Genshin Impact',
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
vi.mock('../../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({ activeGame: fixtures.activeGame }),
}));
vi.mock('../../../hooks/useSettings', () => ({
  useSettings: () => ({
    settings: fixtures.settings,
    saveSettingsAsync: fixtures.saveSettingsAsync,
  }),
}));
vi.mock('../../../lib/bindings', () => ({
  commands: {
    checkPathExistsCmd: vi.fn(),
    resolveGameFolder: vi.fn(),
    reconcileDiskStateCmd: vi.fn(),
  },
}));
vi.mock('../../file-watcher/hooks', () => ({ applyDiskReconcileResult: vi.fn() }));
vi.mock('../../../stores/useToastStore', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));
vi.mock('react-i18next', () => ({
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
    vi.mocked(fixtures.saveSettingsAsync).mockResolvedValue(undefined);
  });

  it('opens on first load when the saved Mods path is missing', async () => {
    renderDialog();

    expect(await screen.findByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('E:/Missing/Mods')).toBeInTheDocument();
  });

  it('resolves a selected root before persisting the relocated Mods path', async () => {
    renderDialog();
    await screen.findByRole('dialog');
    vi.mocked(open).mockResolvedValue('E:/GIMI');

    fireEvent.click(screen.getByRole('button', { name: 'grid:banners.source_locate_btn' }));

    await waitFor(() => expect(commands.resolveGameFolder).toHaveBeenCalledWith('E:/GIMI'));
    expect(fixtures.saveSettingsAsync).toHaveBeenCalledWith({
      ...fixtures.settings,
      games: [{ ...fixtures.activeGame, mod_path: 'E:/GIMI/Mods' }],
    });
  });
});
