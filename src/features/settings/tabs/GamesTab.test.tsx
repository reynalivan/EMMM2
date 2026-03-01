/* eslint-disable @typescript-eslint/no-explicit-any */
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import GamesTab from './GamesTab';
import { useSettings } from '../../../hooks/useSettings';
import { useAppStore } from '../../../stores/useAppStore';
import { useToastStore } from '../../../stores/useToastStore';

// Mock dependencies
vi.mock('../../../hooks/useSettings', () => ({
  useSettings: vi.fn(),
}));

vi.mock('../../../stores/useAppStore', () => ({
  useAppStore: vi.fn(),
}));

vi.mock('../../../stores/useToastStore', () => ({
  useToastStore: vi.fn(),
}));

// Mock the GameFormModal so we don't need to mount it fully for simple tests
vi.mock('../modals/GameFormModal', () => ({
  default: ({ isOpen, onClose, onSave }: any) => {
    if (!isOpen) return null;
    return (
      <div data-testid="game-form-modal">
        <button data-testid="modal-close" onClick={onClose}>
          Close
        </button>
        <button
          data-testid="modal-save"
          onClick={() =>
            onSave({
              id: 'new-id',
              name: 'New Game',
              game_type: 'GIMI',
              mod_path: 'C:/Mods',
              game_exe: 'C:/Game/GenshinImpact.exe',
              loader_exe: 'C:/Game/3dmigotoloader.exe',
            })
          }
        >
          Save
        </button>
      </div>
    );
  },
}));

describe('GamesTab (TC-02)', () => {
  const mockSaveSettings = vi.fn();
  const mockSetActiveGameId = vi.fn();
  const mockAddToast = vi.fn();
  const mockRemoveToast = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mocks
    (useSettings as any).mockReturnValue({
      settings: { games: [] },
      saveSettings: mockSaveSettings,
    });

    (useAppStore as any).mockReturnValue({
      activeGameId: null,
      setActiveGameId: mockSetActiveGameId,
    });

    (useToastStore as any).mockReturnValue({
      addToast: mockAddToast,
      removeToast: mockRemoveToast,
    });
    // For when we check the getState() inside handleRescan
    (useToastStore as any).getState = () => ({
      removeToast: mockRemoveToast,
    });

    window.confirm = vi.fn(() => true);
  });

  it('renders empty state correctly', () => {
    render(<GamesTab />);
    expect(screen.getByText(/No games configured/i)).toBeInTheDocument();
  });

  it('renders a list of games', () => {
    (useSettings as any).mockReturnValue({
      settings: {
        games: [
          {
            id: 'g1',
            name: 'Genshin Impact',
            game_type: 'GIMI',
            mod_path: 'C:/Mods',
            game_exe: 'C:/Game/Genshin.exe',
          },
        ],
      },
      saveSettings: mockSaveSettings,
    });
    (useAppStore as any).mockReturnValue({
      activeGameId: 'g1',
      setActiveGameId: mockSetActiveGameId,
    });

    render(<GamesTab />);
    expect(screen.getByText('Genshin Impact')).toBeInTheDocument();
    expect(screen.getByText('ACTIVE')).toBeInTheDocument();
    expect(screen.getByText('C:/Mods')).toBeInTheDocument();
  });

  it('triggers Add Game flow', () => {
    render(<GamesTab />);

    // Confirm modal is initially closed
    expect(screen.queryByTestId('game-form-modal')).not.toBeInTheDocument();

    // Click add game
    fireEvent.click(screen.getByRole('button', { name: /Add Game/i }));

    // Modal should render
    expect(screen.getByTestId('game-form-modal')).toBeInTheDocument();

    // Simulate saving a new game
    fireEvent.click(screen.getByTestId('modal-save'));

    expect(mockSaveSettings).toHaveBeenCalledWith({
      games: [
        {
          id: 'new-id',
          name: 'New Game',
          game_type: 'GIMI',
          mod_path: 'C:/Mods',
          game_exe: 'C:/Game/GenshinImpact.exe',
          loader_exe: 'C:/Game/3dmigotoloader.exe',
        },
      ],
    });
  });

  it('handles game deletion and unsets activeGameId', () => {
    (useSettings as any).mockReturnValue({
      settings: {
        games: [
          { id: 'g1', name: 'Genshin Impact' },
          { id: 'g2', name: 'Honkai Star Rail' },
        ],
      },
      saveSettings: mockSaveSettings,
    });
    (useAppStore as any).mockReturnValue({
      activeGameId: 'g1',
      setActiveGameId: mockSetActiveGameId,
    });

    render(<GamesTab />);

    // We expect two trash buttons, one for each game. Click the first one (g1)
    const deleteButtons = screen.getAllByTitle('Remove Game');
    fireEvent.click(deleteButtons[0]);

    // confirm is mocked to true
    expect(window.confirm).toHaveBeenCalled();
    expect(mockSaveSettings).toHaveBeenCalledWith({
      games: [{ id: 'g2', name: 'Honkai Star Rail' }], // g1 is removed
    });
    expect(mockSetActiveGameId).toHaveBeenCalledWith(null); // Because g1 was active
  });

  it('handles Set Active game', () => {
    (useSettings as any).mockReturnValue({
      settings: {
        games: [
          { id: 'g1', name: 'Genshin Impact' },
          { id: 'g2', name: 'Honkai Star Rail' },
        ],
      },
      saveSettings: mockSaveSettings,
    });
    (useAppStore as any).mockReturnValue({
      activeGameId: 'g1',
      setActiveGameId: mockSetActiveGameId,
    });

    render(<GamesTab />);

    // Play buttons (Set as Active). First one is disabled (already active).
    const activeButtons = screen.getAllByTitle('Set as Active');
    expect(activeButtons[0]).toBeDisabled();

    // Click second game to make it active
    fireEvent.click(activeButtons[1]);
    expect(mockSetActiveGameId).toHaveBeenCalledWith('g2');
  });
});
