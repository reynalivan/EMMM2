import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import App from './App';
import * as appStore from '@/app/store';
import type { GameConfig } from '@/entities/game';

// Mock the components so we don't render the whole app
vi.mock('@/widgets/app-shell', () => ({
  AppShell: () => <div data-testid="dashboard">Dashboard</div>,
}));
vi.mock('@/pages/onboarding', () => ({
  WelcomeScreen: ({
    onComplete,
  }: {
    onComplete: (
      games: GameConfig[],
      startBackgroundIndexing: () => Promise<void>,
    ) => Promise<void>;
  }) => (
    <button
      data-testid="welcome"
      onClick={() => void onComplete([{ id: 'g1' } as GameConfig], async () => undefined)}
    >
      Welcome
    </button>
  ),
}));
vi.mock('@/widgets/mod-explorer/modals/FolderConflictManager', () => ({
  default: () => <div data-testid="folder-conflict-manager" />,
}));
vi.mock('@/widgets/mod-explorer/modals/RenameConfirmationManager', () => ({
  default: () => <div data-testid="rename-confirmation-manager" />,
}));
vi.mock('@/shared/ui/toast', () => ({
  ToastContainer: () => null,
  useToastStore: () => ({ addToast: vi.fn() }),
}));

// Mock logger
vi.mock('@tauri-apps/plugin-log', () => ({
  trace: vi.fn(),
  info: vi.fn(),
  error: vi.fn(),
  warn: vi.fn(),
  debug: vi.fn(),
  attachConsole: vi.fn(),
}));

describe('App Bootstrap Routing & Initialization (TC-01)', () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Mock the store to prevent initialization errors
    vi.spyOn(appStore.useAppStore.getState(), 'initStore').mockResolvedValue(undefined);
  });

  it('TC-01-08: Routes to /welcome on FreshInstall status', async () => {
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === 'app_startup_check') return Promise.resolve([]);
      if (cmd === 'check_config_status') return Promise.resolve('FreshInstall');
      return Promise.reject(new Error(`Unhandled mock command: ${cmd}`));
    });

    render(
      <MemoryRouter initialEntries={['/']}>
        <App />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('welcome')).toBeInTheDocument();
    });
    expect(invoke).toHaveBeenCalledWith('check_config_status');
    expect(screen.getByTestId('folder-conflict-manager')).toBeInTheDocument();
    expect(screen.getByTestId('rename-confirmation-manager')).toBeInTheDocument();
  });

  it('TC-01-09: Routes to /dashboard on HasConfig status', async () => {
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === 'app_startup_check') return Promise.resolve([]);
      if (cmd === 'check_config_status') return Promise.resolve('HasConfig');
      return Promise.reject(new Error(`Unhandled mock command: ${cmd}`));
    });

    render(
      <MemoryRouter initialEntries={['/']}>
        <App />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('dashboard')).toBeInTheDocument();
    });
    expect(invoke).toHaveBeenCalledWith('check_config_status');
    expect(invoke).not.toHaveBeenCalledWith('check_boot_security', expect.anything());
  });

  it('TC-01-10: Falls back to Welcome on IPC timeout or error', async () => {
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === 'app_startup_check') return Promise.resolve([]);
      if (cmd === 'check_config_status') return Promise.reject(new Error('Backend missing'));
      return Promise.reject(new Error(`Unhandled mock command: ${cmd}`));
    });

    render(
      <MemoryRouter initialEntries={['/']}>
        <App />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('welcome')).toBeInTheDocument();
    });
  });

  it('keeps onboarding visible until the first game activation is ready', async () => {
    vi.mocked(invoke).mockImplementation((cmd) => {
      if (cmd === 'app_startup_check') return Promise.resolve([]);
      if (cmd === 'check_config_status') return Promise.resolve('FreshInstall');
      return Promise.reject(new Error(`Unhandled mock command: ${cmd}`));
    });
    const activate = vi
      .spyOn(appStore.useAppStore.getState(), 'setActiveGameId')
      .mockResolvedValue(undefined);
    appStore.useAppStore.setState({ gameActivationByGame: {} });

    render(
      <MemoryRouter initialEntries={['/']}>
        <App />
      </MemoryRouter>,
    );
    fireEvent.click(await screen.findByTestId('welcome'));
    await waitFor(() =>
      expect(activate).toHaveBeenCalledWith('g1', {
        deferWorkspacePrefetch: true,
        requireActivationStatusListener: true,
      }),
    );
    await Promise.resolve();
    expect(screen.queryByTestId('dashboard')).not.toBeInTheDocument();

    appStore.useAppStore.getState().setGameActivationStatus({
      game_id: 'g1',
      generation: 1,
      phase: 'ready',
      reconcile_revision: 1,
      runtime_sync_generation: null,
      error: null,
    });
    await waitFor(() => expect(screen.getByTestId('dashboard')).toBeInTheDocument());
  });
});
