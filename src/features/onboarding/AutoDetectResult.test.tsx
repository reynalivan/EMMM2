import { render, screen, fireEvent } from '../../testing/test-utils';
import { describe, it, expect, vi } from 'vitest';
import { AutoDetectResult } from './AutoDetectResult';
import type { GameConfig } from '../../types/game';

describe('AutoDetectResult (TC-03)', () => {
  const mockOnContinue = vi.fn();
  const mockOnRemoveGame = vi.fn();
  const mockOnGoBack = vi.fn();

  it('renders games correctly', () => {
    const games = [
      { id: 'g1', name: 'Game 1', game_type: 'GIMI', mod_path: 'C:/Mods1', game_exe: 'C:/G1.exe' },
      { id: 'g2', name: 'Game 2', game_type: 'SRMI', mod_path: 'C:/Mods2', game_exe: 'C:/G2.exe' },
    ] as unknown as GameConfig[];

    render(
      <AutoDetectResult
        games={games}
        onConfirm={mockOnContinue}
        onRemoveGame={mockOnRemoveGame}
        onBack={mockOnGoBack}
      />,
    );

    // Using flexible regex since i18next might return raw keys in test
    expect(screen.getByText(/result\.title/i)).toBeInTheDocument();
    expect(screen.getByText('Game 1')).toBeInTheDocument();
    expect(screen.getByText('Game 2')).toBeInTheDocument();
    // Check for the new big checkmark icon (and the one in the button)
    expect(screen.getAllByTestId('icon-check')).toHaveLength(2);
  });

  it('handles empty state properly', () => {
    render(
      <AutoDetectResult
        games={[]}
        onConfirm={mockOnContinue}
        onRemoveGame={mockOnRemoveGame}
        onBack={mockOnGoBack}
      />,
    );

    expect(screen.getByText(/result\.title/i)).toBeInTheDocument();
  });

  it('triggers callbacks', () => {
    const games = [
      { id: 'g1', name: 'Game 1', game_type: 'GIMI', mod_path: 'C:/Mods', game_exe: 'C:/G.exe' },
    ] as unknown as GameConfig[];
    render(
      <AutoDetectResult
        games={games}
        onConfirm={mockOnContinue}
        onRemoveGame={mockOnRemoveGame}
        onBack={mockOnGoBack}
      />,
    );

    // Testing back to welcome button
    fireEvent.click(screen.getByText(/result\.back_to_welcome/i));
    expect(mockOnGoBack).toHaveBeenCalled();

    // Testing bottom buttons
    fireEvent.click(screen.getByText(/result\.add_another/i));
    expect(mockOnGoBack).toHaveBeenCalledTimes(2);

    fireEvent.click(screen.getByText(/result\.confirm/i));
    expect(mockOnContinue).toHaveBeenCalled();

    // Clicking trash icon
    const removeBtn = screen.getByTitle(/result\.remove_tip/i);
    fireEvent.click(removeBtn);
    expect(mockOnRemoveGame).toHaveBeenCalledWith('g1');
  });
});
