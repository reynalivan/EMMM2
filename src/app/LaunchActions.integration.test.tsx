import { render, within } from '@testing-library/react';
import type { HTMLAttributes } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TopBar } from '@/widgets/top-bar';
import { LaunchBar } from '@/widgets/launch-bar';
import type { ConflictInfo } from '@/entities/workspace';
import type { GameConfig } from '@/entities/game';

const fixture = vi.hoisted(() => ({
  conflicts: [] as ConflictInfo[],
  state: {
    workspaceView: 'mods',
    setWorkspaceView: vi.fn(),
    isAppMenuOpen: false,
    setAppMenuOpen: vi.fn(),
    safetyFilter: 'all',
    setSafetyFilter: vi.fn(),
    autoCloseLauncher: true,
    isPreviewOpen: false,
    togglePreview: vi.fn(),
    requestedGameId: null,
    runtimeSyncByGame: {},
    gameActivationByGame: {},
    diskReconcileByGame: {},
  },
}));

const game: GameConfig = {
  id: 'game-1',
  name: 'GIMI',
  game_type: 0,
  mod_path: 'E:/GIMI/Mods',
  game_exe: 'E:/GIMI/Game.exe',
  loader_exe: null,
  launch_args: null,
};

const unresolvedConflict: ConflictInfo = {
  hash: 'aaaaaaaa',
  section_name: 'TextureOverrideBody',
  mod_paths: ['ModA', 'ModB'],
  is_active: true,
  kind: 'resource_hash',
  certainty: 'definite',
  has_conditional_evidence: false,
  evidence: [],
};

vi.mock('@/entities/game', () => ({
  GAME_OPTIONS: [{ value: '0', label: 'GIMI' }],
  useActiveGame: () => ({ activeGame: game, games: [game], isLoading: false }),
  launchConfiguredGame: vi.fn(),
}));
vi.mock('@/features/mod-runtime', () => ({
  useActiveConflicts: () => ({ data: fixture.conflicts }),
  useIgnoredActiveModConflictGroupKeys: () => ({
    data: [],
    isLoading: false,
    error: null,
    refetch: vi.fn(),
  }),
}));
vi.mock('@/features/workspace-runtime', () => ({
  useBackgroundIndexingStatus: () => ({ sessions: [] }),
  useGameSwitch: () => ({ switchGame: vi.fn() }),
}));
vi.mock('@/app/store', () => ({
  useAppStore: <T,>(selector: (state: typeof fixture.state) => T) => selector(fixture.state),
}));
vi.mock('@tanstack/react-query', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@tanstack/react-query')>()),
  useQueryClient: vi.fn(),
}));
vi.mock('@/features/file-watcher', () => ({ applyDiskReconcileResult: vi.fn() }));
vi.mock('@/shared/ui/toast', () => ({
  toast: { error: vi.fn() },
  useToastStore: { getState: vi.fn() },
}));
vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({
    children,
    liquidRole: _role,
    contentClassName: _content,
    ...props
  }: HTMLAttributes<HTMLDivElement> & { liquidRole?: string; contentClassName?: string }) => (
    <div {...props}>{children}</div>
  ),
}));
vi.mock('@/features/randomizer', () => ({ RandomizerModal: () => null }));
vi.mock('@/features/conflict-report', () => ({
  ConflictModal: () => null,
  buildConflictModSetKey: (paths: string[]) => JSON.stringify([...paths].sort()),
}));

const shell = () => <TopBar launchBar={<LaunchBar />} />;

describe('compact launch actions in the app shell', () => {
  beforeEach(() => {
    fixture.state.workspaceView = 'mods';
    fixture.conflicts = [];
  });

  it('keeps randomize, conflicts, and play available in the compact top-bar menu', () => {
    fixture.conflicts = [unresolvedConflict];
    render(shell());
    const menu = within(document.getElementById('topbar-more-launch-portal')!);

    expect(menu.getByRole('button', { name: 'Play' })).toBeInTheDocument();
    expect(menu.getByRole('button', { name: /Randomizer/ })).toBeInTheDocument();
    expect(menu.getByRole('button', { name: /Enabled Conflicts/ })).toHaveTextContent('1');
  });

  it('mounts compact launch actions after Dashboard changes to Mods', () => {
    fixture.state.workspaceView = 'dashboard';
    fixture.conflicts = [unresolvedConflict];
    const view = render(shell());
    expect(document.getElementById('topbar-more-launch-portal')).toBeNull();

    fixture.state.workspaceView = 'mods';
    view.rerender(shell());

    const menu = within(document.getElementById('topbar-more-launch-portal')!);
    expect(menu.getByRole('button', { name: 'Play' })).toBeInTheDocument();
    expect(menu.getByRole('button', { name: /Randomizer/ })).toBeInTheDocument();
    expect(menu.getByRole('button', { name: /Enabled Conflicts/ })).toHaveTextContent('1');
  });

  it('releases the detached menu and rebinds actions after Mods → Dashboard → Mods', () => {
    const view = render(shell());
    const previousTarget = document.getElementById('topbar-more-launch-portal')!;
    expect(within(previousTarget).getByRole('button', { name: 'Play' })).toBeInTheDocument();

    fixture.state.workspaceView = 'dashboard';
    view.rerender(shell());
    expect(previousTarget.isConnected).toBe(false);
    expect(within(previousTarget).queryByRole('button', { name: 'Play' })).toBeNull();

    fixture.state.workspaceView = 'mods';
    view.rerender(shell());
    const currentTarget = document.getElementById('topbar-more-launch-portal')!;
    expect(currentTarget).not.toBe(previousTarget);
    expect(within(currentTarget).getByRole('button', { name: 'Play' })).toBeInTheDocument();
    expect(within(currentTarget).getByRole('button', { name: /Randomizer/ })).toBeInTheDocument();
  });
});
