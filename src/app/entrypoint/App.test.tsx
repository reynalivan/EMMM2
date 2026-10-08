import { act, render, screen, waitFor } from '@testing-library/react';
import { StrictMode } from 'react';
import { MemoryRouter, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App, { DashboardWorkspace } from './App';

const { state, games, initStore } = vi.hoisted(() => ({
  initStore: vi.fn<() => Promise<void>>(),
  state: {
    workspaceView: 'dashboard',
    selectedObjectFolderPath: null,
    activeGameId: 'game-b' as string | null,
    requestedGameId: null as string | null,
    gameActivationByGame: {
      'game-b': { phase: 'syncing' },
    } as Record<string, { phase: string; error?: string | null }>,
    diskReconcileByGame: {} as Record<string, { progress: null }>,
  },
  games: [
    { id: 'game-a', name: 'Cached game' },
    { id: 'game-b', name: 'Selected game' },
  ],
}));

vi.mock('@/app/store', () => ({
  useAppStore: Object.assign((selector: (value: typeof state) => unknown) => selector(state), {
    getState: () => ({ ...state, initStore }),
  }),
}));
vi.mock('@/entities/settings', () => ({ useSettings: () => ({ settings: undefined }) }));
vi.mock('@/pages/settings', () => ({
  useThemeRuntime: () => undefined,
  DynamicThemeInjector: () => null,
}));
vi.mock('@/shared/lib/logger', () => ({ initLogger: () => Promise.resolve() }));
vi.mock('@/shared/lib/dismissSplash', () => ({ dismissSplash: vi.fn() }));
vi.mock('@/shared/ui/components/ui/DiagnosticsErrorDialog', () => ({
  DiagnosticsErrorDialog: () => null,
}));
vi.mock('@/shared/ui/components/ui/CrashRecoveryDialog', () => ({
  CrashRecoveryDialog: () => null,
}));
vi.mock('@/pages/browser', () => ({ DownloadConfirmationHost: () => null }));
vi.mock('@/widgets/mod-explorer', () => ({
  FolderConflictManager: () => null,
  RenameConfirmationManager: () => null,
  WorkspaceSourceUnavailableDialog: () => null,
}));
vi.mock('@/features/file-watcher', () => ({
  ExternalChangeHandler: () => null,
  FileInUseDialog: () => null,
}));
vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: games[0], games }),
}));
vi.mock('@/features/workspace-runtime', () => ({
  useBackgroundIndexingStatus: () => ({
    gamesById: new Map(),
    snapshotProgressByGame: new Map(),
  }),
  useGameSwitch: () => ({ switchGame: vi.fn() }),
  WorkspaceParentEnableDialogHost: () => null,
}));
vi.mock('@/widgets/app-shell', () => ({
  AppShell: ({ loadingPage }: { loadingPage?: React.ReactNode }) => (
    <div data-testid="workspace">{loadingPage}</div>
  ),
}));
vi.mock('@/widgets/top-bar', () => ({
  GameIndexingOverlay: ({ gameName }: { gameName: string }) => <div role="status">{gameName}</div>,
  TopBar: () => null,
}));
vi.mock('@/shared/lib/appMode', () => ({ isDemoMode: false }));

describe('App startup', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    initStore.mockResolvedValue(undefined);
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'app_startup_check') return [];
      if (command === 'check_config_status') return 'HasConfig';
      throw new Error(`Unexpected startup command: ${command}`);
    });
  });

  it('runs boot once across StrictMode replay and subsequent route changes', async () => {
    let finishRecovery!: (tasks: []) => void;
    const recovery = new Promise<[]>((resolve) => {
      finishRecovery = resolve;
    });
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'app_startup_check') return recovery;
      if (command === 'check_config_status') return 'HasConfig';
      throw new Error(`Unexpected startup command: ${command}`);
    });
    let navigate!: ReturnType<typeof useNavigate>;
    function NavigationControl() {
      navigate = useNavigate();
      return null;
    }

    render(
      <StrictMode>
        <MemoryRouter initialEntries={['/']}>
          <NavigationControl />
          <App />
        </MemoryRouter>
      </StrictMode>,
    );
    expect(
      vi.mocked(invoke).mock.calls.filter(([command]) => command === 'app_startup_check'),
    ).toHaveLength(1);

    await act(async () => finishRecovery([]));
    await waitFor(() => expect(screen.getByTestId('workspace')).toBeInTheDocument());
    await act(async () => navigate('/unknown-route'));
    await waitFor(() => expect(screen.getByTestId('workspace')).toBeInTheDocument());

    expect(initStore).toHaveBeenCalledTimes(1);
    expect(
      vi.mocked(invoke).mock.calls.filter(([command]) => command === 'app_startup_check'),
    ).toHaveLength(1);
    expect(
      vi.mocked(invoke).mock.calls.filter(([command]) => command === 'check_config_status'),
    ).toHaveLength(1);
  });
});

describe('DashboardWorkspace readiness', () => {
  beforeEach(() => {
    state.workspaceView = 'dashboard';
    state.activeGameId = 'game-b';
    state.requestedGameId = null;
    state.gameActivationByGame = { 'game-b': { phase: 'syncing' } };
  });

  it('gates the store active game while the settings cache points to another game', () => {
    render(<DashboardWorkspace />);
    expect(screen.getByRole('status')).toHaveTextContent('Selected game');
  });

  it('gates the requested game while its activation request is pending', () => {
    state.activeGameId = 'game-a';
    state.requestedGameId = 'game-b';
    state.gameActivationByGame['game-a'] = { phase: 'ready' };
    render(<DashboardWorkspace />);
    expect(screen.getByRole('status')).toHaveTextContent('Selected game');
  });

  it('gates the active ID even before its game appears in the settings cache', () => {
    state.activeGameId = 'game-c';
    render(<DashboardWorkspace />);
    expect(screen.getByRole('status')).toHaveTextContent('game-c');
  });

  it('shows the workspace as soon as activation is ready during background indexing', () => {
    state.gameActivationByGame['game-b'] = { phase: 'ready' };
    render(<DashboardWorkspace />);
    expect(screen.queryByRole('status')).toBeNull();
  });
});
