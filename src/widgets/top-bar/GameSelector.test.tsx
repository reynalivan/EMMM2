import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { ReactNode } from 'react';
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
    vi.clearAllMocks();
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

  it('calls switchGame with UUID when a game is selected', () => {
    render(<GameSelector />);

    const starRailBtn = screen.getByText('Star Rail');
    fireEvent.click(starRailBtn);

    expect(mockSwitchGame).toHaveBeenCalledWith('uuid-srmi');
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
});
