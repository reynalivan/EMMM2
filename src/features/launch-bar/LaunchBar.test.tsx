import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import LaunchBar from './LaunchBar';
import { invoke } from '@tauri-apps/api/core';
import { exit } from '@tauri-apps/plugin-process';

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: vi.fn(() => ({ activeGame: { id: 'game-1' } })),
}));
vi.mock('../../hooks/useFolders', () => ({
  useActiveConflicts: vi.fn(() => ({ data: [] })), // no conflicts initially
}));
vi.mock('../../stores/useAppStore', () => ({
  useAppStore: vi.fn(() => ({ autoCloseLauncher: true })),
}));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));
vi.mock('@tauri-apps/plugin-process', () => ({
  exit: vi.fn(),
}));

// Mock inner modals so they don't break rendering
vi.mock('../randomizer/RandomizerModal', () => ({
  default: () => <div data-testid="randomizer-modal"></div>,
}));
vi.mock('../conflict-report/ConflictModal', () => ({
  default: () => <div data-testid="conflict-modal"></div>,
}));
vi.mock('../scanner/components/ConflictToast', () => ({
  default: () => <div data-testid="conflict-toast"></div>,
}));

describe('LaunchBar', () => {
  it('launches game and triggers exit if autoClose is true', async () => {
    render(<LaunchBar />);

    fireEvent.click(screen.getByText('Play'));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('launch_game', { gameId: 'game-1' });
    });

    expect(exit).toHaveBeenCalledWith(0);
  });

  it('handles launch error gracefully', async () => {
    vi.mocked(invoke).mockRejectedValue(new Error('Launch failed'));
    render(<LaunchBar />);

    fireEvent.click(screen.getByText('Play'));

    await waitFor(() => {
      expect(screen.getByText(/Launch failed/)).toBeInTheDocument();
    });
  });
});
