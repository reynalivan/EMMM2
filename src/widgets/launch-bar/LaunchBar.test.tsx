import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, it, expect, vi } from 'vitest';
import LaunchBar from './LaunchBar';
import type { ConflictInfo } from '@/entities/workspace';

const launchConfiguredGame = vi.fn();
const toastError = vi.fn();
let activeConflicts: ConflictInfo[] = [];
let workspaceView = 'mods';

vi.mock('@/entities/game', () => ({
  useActiveGame: vi.fn(() => ({ activeGame: { id: 'game-1' } })),
  launchConfiguredGame: (...args: unknown[]) => launchConfiguredGame(...args),
}));
vi.mock('@/features/mod-runtime', () => ({
  useActiveConflicts: vi.fn(() => ({ data: activeConflicts })),
}));
vi.mock('@/app/store', () => ({
  useAppStore: (
    selector: (state: { autoCloseLauncher: boolean; workspaceView: string }) => unknown,
  ) => selector({ autoCloseLauncher: true, workspaceView }),
}));
vi.mock('@/shared/ui/toast', () => ({
  toast: { error: (...args: unknown[]) => toastError(...args) },
}));
vi.mock('react-i18next', () => ({
  initReactI18next: { type: '3rdParty', init: vi.fn() },
  useTranslation: () => ({
    t: (key: string) => {
      const labels: Record<string, string> = {
        'layout:launch_bar.play': 'Play',
        'layout:launch_bar.launching': 'Launching',
        'layout:launch_bar.randomizer': 'Randomizer',
        'layout:launch_bar.conflicts': 'Conflicts',
        'layout:launch_bar.shared_hashes': 'Shared hashes',
      };

      return labels[key] ?? key;
    },
  }),
}));
// Mock inner modals so they don't break rendering
vi.mock('@/features/randomizer', () => ({
  RandomizerModal: () => <div data-testid="randomizer-modal"></div>,
}));
vi.mock('@/features/conflict-report', () => ({
  ConflictModal: () => <div data-testid="conflict-modal"></div>,
}));
vi.mock('@/features/scanner', () => ({
  ConflictToast: ({ onDismiss }: { onDismiss: () => void }) => (
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
    workspaceView = 'mods';
    launchConfiguredGame.mockReset();
    toastError.mockReset();
  });

  it('launches through the shared action with auto-close enabled', async () => {
    render(<LaunchBar />);

    fireEvent.click(screen.getByText('Play'));

    await waitFor(() => {
      expect(launchConfiguredGame).toHaveBeenCalledWith('game-1', true);
    });
  });

  it('reports a launch error through the app toast without an inline alert', async () => {
    launchConfiguredGame.mockRejectedValue(new Error('Launch failed'));
    render(<LaunchBar />);

    fireEvent.click(screen.getByText('Play'));

    await waitFor(() => {
      expect(toastError).toHaveBeenCalledWith('Launch failed');
    });

    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
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

  it('presents shared hash conflicts as a quiet review action', () => {
    activeConflicts = [conflict('aaaaaaaa')];
    render(<LaunchBar />);

    const trigger = screen.getByRole('button', { name: /shared hashes/i });
    expect(trigger).toHaveClass('btn-ghost');
    expect(trigger).toHaveClass('text-info');
    expect(trigger).not.toHaveClass('btn-warning');
    expect(trigger).not.toHaveClass('animate-pulse');
    expect(trigger).toHaveTextContent('1');
  });

  it('keeps randomize, conflicts, and play available in the compact top-bar menu', async () => {
    const target = document.createElement('li');
    target.id = 'topbar-more-launch-portal';
    document.body.appendChild(target);
    activeConflicts = [conflict('aaaaaaaa')];

    const { unmount } = render(<LaunchBar />);
    const menu = within(target);

    await waitFor(() => expect(menu.getByRole('button', { name: 'Play' })).toBeInTheDocument());
    expect(menu.getByRole('button', { name: 'Randomizer' })).toBeInTheDocument();
    expect(menu.getByRole('button', { name: /Conflicts/ })).toHaveTextContent('1');

    unmount();
    target.remove();
  });

  it('hides the randomizer outside Mods Manager', () => {
    workspaceView = 'dashboard';
    render(<LaunchBar />);

    expect(screen.queryByTitle('Randomizer')).not.toBeInTheDocument();
  });
});
