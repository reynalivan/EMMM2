import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import GameSelector from './GameSelector';

// Mock useActiveGame hook
const mockActiveGame = {
  id: 'uuid-gimi',
  name: 'Genshin Impact',
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

vi.mock('../../../hooks/useActiveGame', () => ({
  useActiveGame: () => ({
    activeGame: mockActiveGame,
    games: mockGames,
    isLoading: false,
    error: null,
  }),
}));

// Mock useGameSwitch hook
const mockSwitchGame = vi.fn();
vi.mock('../../../hooks/useObjects', () => ({
  useGameSwitch: () => ({
    switchGame: mockSwitchGame,
  }),
}));

describe('GameSelector', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders correctly with active game name', () => {
    render(<GameSelector />);
    const elements = screen.getAllByText('Genshin Impact');
    expect(elements.length).toBeGreaterThan(0);
  });

  it('shows game type badge', () => {
    render(<GameSelector />);
    // The active game badge shows the game_type
    const badges = screen.getAllByText('GIMI');
    expect(badges.length).toBeGreaterThan(0);
  });

  it('renders all games in dropdown', () => {
    render(<GameSelector />);
    const giElements = screen.getAllByText('Genshin Impact');
    expect(giElements.length).toBeGreaterThan(0);
    expect(screen.getByText('Star Rail')).toBeInTheDocument();
  });

  it('calls switchGame with UUID when a game is selected', () => {
    render(<GameSelector />);

    const starRailBtn = screen.getByText('Star Rail');
    fireEvent.click(starRailBtn);

    expect(mockSwitchGame).toHaveBeenCalledWith('uuid-srmi');
  });
});
