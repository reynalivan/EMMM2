import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DashboardWorkspace } from './App';

const { state, games } = vi.hoisted(() => ({
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
  useAppStore: (selector: (value: typeof state) => unknown) => selector(state),
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

describe('DashboardWorkspace readiness', () => {
  beforeEach(() => {
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
