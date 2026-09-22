import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { ReactNode } from 'react';
import type { OnboardingIndexingBackgroundGameStatus } from '@/shared/api/tauri/bindings';
import GameSelector from './GameSelector';

const mockBackgroundIndexingState = vi.hoisted(() => ({
  isLoaded: true,
  loadError: false,
  sessions: [],
  gamesById: new Map<string, OnboardingIndexingBackgroundGameStatus>(),
  refresh: vi.fn().mockResolvedValue(undefined),
}));

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

vi.mock('@/pages/onboarding/hooks/useBackgroundIndexingStatus', () => ({
  useBackgroundIndexingStatus: () => mockBackgroundIndexingState,
}));

describe('GameSelector', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockBackgroundIndexingState.isLoaded = true;
    mockBackgroundIndexingState.loadError = false;
    mockBackgroundIndexingState.gamesById.clear();
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

  it('shows loading while the selected game is indexing', async () => {
    let finishSwitch: () => void = () => undefined;
    mockSwitchGame.mockReturnValue(
      new Promise<void>((resolve) => {
        finishSwitch = resolve;
      }),
    );

    render(<GameSelector />);
    fireEvent.click(screen.getByText('Star Rail'));

    expect(screen.getByText('Loading...')).toBeInTheDocument();

    finishSwitch();
    await waitFor(() => expect(screen.queryByText('Loading...')).not.toBeInTheDocument());
  });

  it('waits for background indexing before switching to a queued game', async () => {
    mockBackgroundIndexingState.gamesById.set('uuid-srmi', {
      game_id: 'uuid-srmi',
      phase: 'Preparing',
    });
    const { rerender } = render(<GameSelector />);

    fireEvent.click(screen.getByText('Star Rail'));
    expect(mockSwitchGame).not.toHaveBeenCalled();
    expect(screen.getByText('Star Rail is still indexing')).toBeInTheDocument();

    mockBackgroundIndexingState.gamesById.set('uuid-srmi', {
      game_id: 'uuid-srmi',
      phase: 'Ready',
    });
    rerender(<GameSelector />);

    await waitFor(() => expect(mockSwitchGame).toHaveBeenCalledWith('uuid-srmi'));
  });

  it('starts a full recheck immediately when background indexing needs attention', async () => {
    mockBackgroundIndexingState.gamesById.set('uuid-srmi', {
      game_id: 'uuid-srmi',
      phase: 'NeedsAttention',
    });
    render(<GameSelector />);

    fireEvent.click(screen.getByText('Star Rail'));

    await waitFor(() => expect(mockSwitchGame).toHaveBeenCalledWith('uuid-srmi'));
    expect(screen.queryByText('Star Rail needs attention')).toBeNull();
  });

  it('does not switch when indexing status cannot be verified', () => {
    mockBackgroundIndexingState.loadError = true;
    render(<GameSelector />);

    fireEvent.click(screen.getByText('Star Rail'));

    expect(mockSwitchGame).not.toHaveBeenCalled();
    expect(screen.getByText('Cannot verify Star Rail yet')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Check again'));
    expect(mockBackgroundIndexingState.refresh).toHaveBeenCalledOnce();
  });
});
