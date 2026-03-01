import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import AutoDetectResult from './AutoDetectResult';
import type { GameConfig } from '../../types/game';

describe('AutoDetectResult (TC-03)', () => {
  const mockOnContinue = vi.fn();
  const mockOnAddMore = vi.fn();
  const mockOnRemoveGame = vi.fn();

  it('renders games correctly', () => {
    const games = [
      { id: 'g1', name: 'Game 1', game_type: 'GIMI', mod_path: 'C:/Mods1', game_exe: 'C:/G1.exe' },
      { id: 'g2', name: 'Game 2', game_type: 'SRMI', mod_path: 'C:/Mods2', game_exe: 'C:/G2.exe' },
    ] as unknown as GameConfig[];

    render(
      <AutoDetectResult
        games={games}
        onContinue={mockOnContinue}
        onAddMore={mockOnAddMore}
        onRemoveGame={mockOnRemoveGame}
      />,
    );

    expect(screen.getByText('2 Games Found!')).toBeInTheDocument();
    expect(screen.getByText('Game 1')).toBeInTheDocument();
    expect(screen.getByText('Game 2')).toBeInTheDocument();
    expect(screen.getByText('GIMI')).toBeInTheDocument();
    expect(screen.getByText('SRMI')).toBeInTheDocument();
  });

  it('handles empty state properly', () => {
    render(
      <AutoDetectResult
        games={[]}
        onContinue={mockOnContinue}
        onAddMore={mockOnAddMore}
        onRemoveGame={mockOnRemoveGame}
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
        onContinue={mockOnContinue}
        onAddMore={mockOnAddMore}
        onRemoveGame={mockOnRemoveGame}
      />,
    );

    fireEvent.click(screen.getByText(/Add Another/i));
    expect(mockOnAddMore).toHaveBeenCalled();

    fireEvent.click(screen.getByText(/Confirm/i));
    expect(mockOnContinue).toHaveBeenCalled();

    // Clicking trash icon
    const removeBtn = screen.getByTitle('Remove from detection');
    fireEvent.click(removeBtn);
    expect(mockOnRemoveGame).toHaveBeenCalledWith('g1');
  });
});
