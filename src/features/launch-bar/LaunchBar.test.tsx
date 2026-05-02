import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import LaunchBar from './LaunchBar';
import { exit } from '@tauri-apps/plugin-process';

const launchGame = vi.fn();

vi.mock('../../hooks/useActiveGame', () => ({
  useActiveGame: vi.fn(() => ({ activeGame: { id: 'game-1' } })),
}));
vi.mock('../../hooks/useFolderMutations', () => ({
  useActiveConflicts: vi.fn(() => ({ data: [] })), // no conflicts initially
}));
vi.mock('../../stores/useAppStore', () => ({
  useAppStore: vi.fn(() => ({ autoCloseLauncher: true })),
}));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const labels: Record<string, string> = {
        'layout:launch_bar.play': 'Play',
        'layout:launch_bar.launching': 'Launching',
        'layout:launch_bar.randomizer': 'Randomizer',
        'layout:launch_bar.conflicts': 'Conflicts',
      };

      return labels[key] ?? key;
    },
  }),
}));
vi.mock('../../lib/bindings', () => ({
  commands: {
    launchGame: (...args: unknown[]) => launchGame(...args),
  },
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
      expect(launchGame).toHaveBeenCalledWith({ gameId: 'game-1' });
    });

    expect(exit).toHaveBeenCalledWith(0);
  });

  it('handles launch error gracefully', async () => {
    launchGame.mockRejectedValue(new Error('Launch failed'));
    render(<LaunchBar />);

    fireEvent.click(screen.getByText('Play'));

    await waitFor(() => {
      expect(screen.getByText(/Launch failed/)).toBeInTheDocument();
    });
  });
});
