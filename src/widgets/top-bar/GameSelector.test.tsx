import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { ReactNode } from 'react';
import { useAppStore } from '@/app/store';
import GameSelector from './GameSelector';

vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

// Mock useActiveGame hook
const mockActiveGame = {
  id: 'uuid-gimi',
  name: 'GIMI',
  game_type: 'GIMI',
  path: 'C:\\Games\\GIMI',
  mods_path: 'C:\\Games\\GIMI\\Mods',
  launcher_path: '',
  launch_args: null,
};
const mockGames = [
  mockActiveGame,
  {
    id: 'uuid-srmi',
    name: 'Star Rail',
    game_type: 'SRMI',
    path: 'C:\\Games\\SRMI',
    mods_path: 'C:\\Games\\SRMI\\Mods',
    launcher_path: '',
    launch_args: null,
  },
];

vi.mock('@/entities/game', () => ({
  GAME_OPTIONS: [
    { value: 'GIMI', label: 'GIMI' },
    { value: 'SRMI', label: 'SRMI' },
  ],
  useActiveGame: () => ({
    activeGame: mockActiveGame,
    games: mockGames,
    isLoading: false,
    error: null,
  }),
}));

// Mock useGameSwitch hook
const mockSwitchGame = vi.fn();
vi.mock('@/features/workspace-runtime', () => ({
  useGameSwitch: () => ({
    switchGame: mockSwitchGame,
  }),
}));

describe('GameSelector', () => {
  beforeEach(() => {
    mockSwitchGame.mockReset();
    mockSwitchGame.mockResolvedValue(undefined);
    useAppStore.setState({ requestedGameId: null, gameActivationByGame: {} });
  });

  it('combines the app identity with the active game label', () => {
    render(<GameSelector />);
    expect(screen.getByText('EMMM')).toBeInTheDocument();
    const elements = screen.getAllByText('GIMI');
    expect(elements.length).toBeGreaterThan(0);
  });

  it('renders all games in dropdown', () => {
    render(<GameSelector />);
    const giElements = screen.getAllByText('GIMI');
    expect(giElements.length).toBeGreaterThan(0);
    expect(screen.getByText('Star Rail')).toBeInTheDocument();
  });

  it('calls switchGame with UUID when a game is selected', async () => {
    render(<GameSelector />);

    const starRailBtn = screen.getByText('Star Rail');
    fireEvent.click(starRailBtn);

    await waitFor(() => expect(mockSwitchGame).toHaveBeenCalledWith('uuid-srmi'));
  });

  it('starts activation immediately even when background indexing is still running', async () => {
    render(<GameSelector />);
    fireEvent.click(screen.getByText('Star Rail'));
    await waitFor(() => expect(mockSwitchGame).toHaveBeenCalledWith('uuid-srmi'));
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('allows the latest selection while a previous activation is pending', async () => {
    mockSwitchGame.mockImplementation((gameId: string) => {
      useAppStore.setState({ requestedGameId: gameId });
      return new Promise<void>(() => undefined);
    });
    render(<GameSelector />);
    fireEvent.click(screen.getByText('Star Rail'));
    const gimiLabels = screen.getAllByText('GIMI');
    fireEvent.click(gimiLabels[gimiLabels.length - 1]);
    expect(mockSwitchGame.mock.calls.map(([gameId]) => gameId)).toEqual(['uuid-srmi', 'uuid-gimi']);
  });

  it('retries activation for an active game that failed indexing', async () => {
    useAppStore.setState({
      gameActivationByGame: {
        'uuid-gimi': {
          game_id: 'uuid-gimi',
          generation: 1,
          phase: 'failed',
          reconcile_revision: null,
          runtime_sync_generation: null,
          error: 'Indexing failed',
        },
      },
    });
    render(<GameSelector />);
    fireEvent.click(screen.getByText('Retry'));
    expect(mockSwitchGame).toHaveBeenCalledWith('uuid-gimi');
  });
});
