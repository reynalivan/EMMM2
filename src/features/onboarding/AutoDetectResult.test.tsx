import { render, screen, fireEvent } from '../../testing/test-utils';
import { describe, it, expect, vi } from 'vitest';
import { AutoDetectResult } from './AutoDetectResult';
import type { GameConfig } from '../../types/game';

describe('AutoDetectResult (TC-03)', () => {
  const mockOnContinue = vi.fn();
  const mockOnAddMore = vi.fn();
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
        onAddMore={mockOnAddMore}
        onRemoveGame={mockOnRemoveGame}
        onBack={mockOnGoBack}
      />,
    );

    expect(screen.getByText('2 Games Found!')).toBeInTheDocument();
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
        onAddMore={mockOnAddMore}
        onRemoveGame={mockOnRemoveGame}
        onBack={mockOnGoBack}
      />,
    );

    expect(screen.getByText('0 Games Found!')).toBeInTheDocument();
  });

  it('triggers callbacks', () => {
    const games = [
      { id: 'g1', name: 'Game 1', game_type: 'GIMI', mod_path: 'C:/Mods', game_exe: 'C:/G.exe' },
    ] as unknown as GameConfig[];
    render(
      <AutoDetectResult
        games={games}
        onConfirm={mockOnContinue}
        onAddMore={mockOnAddMore}
        onRemoveGame={mockOnRemoveGame}
        onBack={mockOnGoBack}
      />,
    );

    fireEvent.click(screen.getByText('Back to Welcome'));
    expect(mockOnGoBack).toHaveBeenCalled();

    fireEvent.click(screen.getByText('Add Another'));
    expect(mockOnAddMore).toHaveBeenCalled();

    fireEvent.click(screen.getByText('Confirm'));
    expect(mockOnContinue).toHaveBeenCalled();

    const removeBtn = screen.getByTitle('Remove from detection');
    fireEvent.click(removeBtn);
    expect(mockOnRemoveGame).toHaveBeenCalledWith('g1');
  });
});
