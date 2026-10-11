import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { beforeEach, describe, it, expect, vi } from 'vitest';
import LaunchBar from './LaunchBar';
import type { ConflictInfo } from '@/entities/workspace';

const launchConfiguredGame = vi.fn();
const toastError = vi.fn();
let activeConflicts: ConflictInfo[] = [];
let ignoredConflictGroupKeys: string[] = [];
let workspaceView = 'mods';

vi.mock('@/entities/game', () => ({
  useActiveGame: vi.fn(() => ({ activeGame: { id: 'game-1' } })),
  launchConfiguredGame: (...args: unknown[]) => launchConfiguredGame(...args),
}));
vi.mock('@/features/mod-runtime', () => ({
  useActiveConflicts: vi.fn(() => ({ data: activeConflicts })),
  useIgnoredActiveModConflictGroupKeys: vi.fn(() => ({
    data: ignoredConflictGroupKeys,
    isLoading: false,
    error: null,
    refetch: vi.fn(),
  })),
}));
vi.mock('@/app/store', () => ({
  useAppStore: <T,>(
    selector: (state: {
      autoCloseLauncher: boolean;
      workspaceView: string;
      isPreviewOpen: boolean;
      togglePreview: () => void;
      diskReconcileByGame: Record<string, never>;
    }) => T,
  ) =>
    selector({
      autoCloseLauncher: true,
      workspaceView,
      isPreviewOpen: false,
      togglePreview: vi.fn(),
      diskReconcileByGame: {},
    }),
}));
vi.mock('@/shared/ui/toast', () => ({
  toast: { error: (...args: unknown[]) => toastError(...args) },
  useToastStore: { getState: vi.fn() },
}));
vi.mock('@tanstack/react-query', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@tanstack/react-query')>()),
  useQueryClient: vi.fn(),
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
  buildConflictModSetKey: (modPaths: string[]) => JSON.stringify([...modPaths].sort()),
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
    ignoredConflictGroupKeys = [];
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

  it('shows a warning trigger without an automatic conflict overlay', () => {
    activeConflicts = [conflict('aaaaaaaa')];
    render(<LaunchBar />);

    expect(screen.queryByText('Dismiss conflict')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: /shared hashes/i })).toBeInTheDocument();
  });

  it('presents shared hash conflicts as a quiet review action', () => {
    activeConflicts = [conflict('aaaaaaaa')];
    render(<LaunchBar />);

    const trigger = screen.getByRole('button', { name: /shared hashes/i });
    expect(trigger).toHaveClass('btn-ghost');
    expect(trigger).toHaveClass('text-warning');
    expect(trigger).not.toHaveClass('text-info');
    expect(trigger).not.toHaveClass('animate-pulse');
    expect(trigger).toHaveTextContent('1');
  });

  it('hides ignored conflict groups from the warning count', () => {
    activeConflicts = [conflict('aaaaaaaa')];
    ignoredConflictGroupKeys = ['["ModA","ModB"]'];
    render(<LaunchBar />);

    expect(screen.queryByRole('button', { name: /shared hashes/i })).not.toBeInTheDocument();
  });

  it('hides the randomizer outside Mods Manager', () => {
    workspaceView = 'dashboard';
    render(<LaunchBar />);

    expect(screen.queryByTitle('Randomizer')).not.toBeInTheDocument();
  });
});
