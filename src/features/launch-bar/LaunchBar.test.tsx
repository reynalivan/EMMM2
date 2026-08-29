import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { beforeEach, describe, it, expect, vi } from 'vitest';
import LaunchBar from './LaunchBar';
import { exit } from '@tauri-apps/plugin-process';
import type { ConflictInfo } from '../../types/scanner';

const launchGame = vi.fn();
let activeConflicts: ConflictInfo[] = [];

vi.mock('../dashboard/hooks/useActiveGame', () => ({
  useActiveGame: vi.fn(() => ({ activeGame: { id: 'game-1' } })),
}));
vi.mock('../folder-grid/hooks/useFolderMutations', () => ({
  useActiveConflicts: vi.fn(() => ({ data: activeConflicts })),
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
vi.mock('../../core/tauri/bindings', () => ({
  sparse: (value: unknown) => value,
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
  default: ({ onDismiss }: { onDismiss: () => void }) => (
    <button data-testid="conflict-toast" onClick={onDismiss}>
      Dismiss conflict
    </button>
  ),
}));

function conflict(hash: string): ConflictInfo {
  return {
    hash,
    section_name: 'TextureOverrideBody',
    mod_paths: ['ModA', 'ModB'],
    is_active: true,
    kind: 'resource_hash',
    certainty: 'definite',
    has_conditional_evidence: false,
    evidence: [],
  };
}

describe('LaunchBar', () => {
  beforeEach(() => {
    activeConflicts = [];
    launchGame.mockReset();
  });

  it('launches game and triggers exit if autoClose is true', async () => {
    render(<LaunchBar />);

    fireEvent.click(screen.getByText('Play'));

    await waitFor(() => {
      expect(launchGame).toHaveBeenCalledWith('game-1');
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

  it('shows a changed conflict batch after the previous batch was dismissed', () => {
    activeConflicts = [conflict('aaaaaaaa')];
    const { rerender } = render(<LaunchBar />);

    fireEvent.click(screen.getByTestId('conflict-toast'));
    expect(screen.queryByTestId('conflict-toast')).not.toBeInTheDocument();

    activeConflicts = [conflict('bbbbbbbb')];
    rerender(<LaunchBar />);

    expect(screen.getByTestId('conflict-toast')).toBeInTheDocument();
  });
});
