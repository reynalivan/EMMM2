import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ComponentProps, HTMLAttributes } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TopBar } from './index';

const mockSetWorkspaceView = vi.fn();
const mockSetSettingsTab = vi.fn();
const mockSetAppMenuOpen = vi.fn();
let workspaceView = 'dashboard';
let activeGame: { id: string; game_exe?: string | null; launch_mode?: string } | null = null;
let runtimeSyncByGame: Record<string, Record<string, unknown>> = {};
const { mockRetryRuntimeSync } = vi.hoisted(() => ({ mockRetryRuntimeSync: vi.fn() }));

vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      workspaceView,
      setWorkspaceView: mockSetWorkspaceView,
      setSettingsTab: mockSetSettingsTab,
      isAppMenuOpen: false,
      setAppMenuOpen: mockSetAppMenuOpen,
      safetyFilter: 'all',
      setSafetyFilter: vi.fn(),
      autoCloseLauncher: false,
      runtimeSyncByGame,
    }),
}));

vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame }),
  requiresGameExecutable: (game: { game_exe?: string | null; launch_mode?: string }) =>
    game.launch_mode !== 'xxmi_managed' && !game.game_exe?.trim(),
}));

vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: {
    launchGame: vi.fn(),
    retryRuntimeSync: mockRetryRuntimeSync,
    getOnboardingIndexingBackgroundStatus: vi.fn().mockResolvedValue([]),
  },
}));
vi.mock('./GameSelector', () => ({ default: () => null }));
vi.mock('./ContextControls', () => ({ default: () => null }));
vi.mock('./GlobalActions', () => ({ default: () => null }));
vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({
    children,
    className,
    liquidRole: _liquidRole,
    contentClassName: _contentClassName,
    ...props
  }: HTMLAttributes<HTMLDivElement> & { liquidRole?: string; contentClassName?: string }) => (
    <div className={className} {...props}>
      {children}
    </div>
  ),
}));

async function renderTopBar(props: ComponentProps<typeof TopBar> = {}) {
  let view!: ReturnType<typeof render>;
  await act(async () => {
    view = render(<TopBar {...props} />);
  });
  return view;
}

describe('TopBar app menu', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    workspaceView = 'dashboard';
    activeGame = null;
    runtimeSyncByGame = {};
    mockSetSettingsTab.mockReset();
  });

  it('navigates to Mod Inbox from the app menu', async () => {
    await renderTopBar();

    fireEvent.click(screen.getByTitle('App Menu'));
    fireEvent.click(screen.getByTestId('nav-mod-inbox'));

    expect(mockSetWorkspaceView).toHaveBeenCalledWith('mod-inbox');
  });

  it('includes Discover and Downloads in the dashboard action order', async () => {
    await renderTopBar();

    fireEvent.click(screen.getByTitle('App Menu'));

    const itemIds = screen
      .getAllByRole('button')
      .map((button) => button.getAttribute('data-testid'))
      .filter((id): id is string => id !== null && id.startsWith('nav-'));

    expect(itemIds).toEqual([
      'nav-dashboard',
      'nav-mods',
      'nav-mod-inbox',
      'nav-collections',
      'nav-storage-optimizer',
      'nav-browser',
      'nav-downloads',
      'nav-settings',
    ]);
  });

  it('keeps the back action inside the app menu', async () => {
    workspaceView = 'mods';
    await renderTopBar();

    expect(screen.queryByTitle('Back to Dashboard')).not.toBeInTheDocument();

    fireEvent.click(screen.getByTitle('App Menu'));

    expect(screen.getByTestId('app-menu-overlay')).toHaveClass('app-menu-overlay');
    fireEvent.click(screen.getByTestId('nav-dashboard'));

    expect(mockSetWorkspaceView).toHaveBeenCalledWith('dashboard');
  });

  it('renders the app menu outside the topbar clipping surface', async () => {
    await renderTopBar();

    fireEvent.click(screen.getByTitle('App Menu'));

    const overlay = screen.getByTestId('app-menu-overlay');
    expect(overlay.parentElement?.parentElement).toBe(document.body);
    expect(overlay.parentElement).toHaveClass('fixed');
  });

  it('publishes the app-menu state while the menu is open', async () => {
    await renderTopBar();

    fireEvent.click(screen.getByTitle('App Menu'));
    expect(mockSetAppMenuOpen).toHaveBeenCalledWith(true);

    fireEvent.click(screen.getByTitle('App Menu'));
    expect(mockSetAppMenuOpen).toHaveBeenCalledWith(false);
  });

  it('shows collection controls only in Mods Manager', async () => {
    const contextControls = <span>Collection controls</span>;
    const { rerender } = await renderTopBar({ contextControls });

    expect(screen.queryByText('Collection controls')).not.toBeInTheDocument();

    workspaceView = 'mods';
    rerender(<TopBar contextControls={contextControls} />);

    expect(screen.getByText('Collection controls')).toBeInTheDocument();
    expect(screen.getByTestId('topbar-center')).toHaveClass(
      'xl:absolute',
      'xl:left-1/2',
      'xl:-translate-x-1/2',
    );
    expect(document.getElementById('topbar-actions-portal')).toHaveClass('xl:ml-auto');
  });

  it('uses the top bar title for Discover', async () => {
    workspaceView = 'browser';
    await renderTopBar();

    expect(screen.getByText('Discover')).toBeInTheDocument();
  });

  it('requeues a failed runtime sync without blocking navigation', async () => {
    workspaceView = 'mods';
    activeGame = { id: 'game-1' };
    runtimeSyncByGame = {
      'game-1': {
        game_id: 'game-1',
        generation: 4,
        phase: 'failed',
        cause: 'effective_mods_changed',
        message: 'injected publication failure',
      },
    };
    mockRetryRuntimeSync.mockResolvedValue(5);
    await renderTopBar();

    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    });

    await waitFor(() => expect(mockRetryRuntimeSync).toHaveBeenCalledWith('game-1'));
    expect(screen.getByTestId('topbar-center')).toBeInTheDocument();
  });

  it('does not expose the internal focus validation message for a manual reload', async () => {
    activeGame = { id: 'game-1' };
    runtimeSyncByGame = {
      'game-1': {
        game_id: 'game-1',
        generation: 4,
        phase: 'needs_manual_reload',
        cause: 'game_activated',
        message: 'Validation error: NeedsManualReload: active game is not focused',
      },
    };

    await renderTopBar();

    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(screen.queryByText(/NeedsManualReload/)).not.toBeInTheDocument();
  });

  it('does not display a manual reload hint when the executable is omitted', async () => {
    activeGame = { id: 'game-1', game_exe: null };
    runtimeSyncByGame = {
      'game-1': {
        game_id: 'game-1',
        generation: 4,
        phase: 'needs_manual_reload',
        cause: 'effective_mods_changed',
        reload_binding: 'Ctrl+F11',
      },
    };
    await renderTopBar();
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Retry' })).not.toBeInTheDocument();
    expect(mockRetryRuntimeSync).not.toHaveBeenCalled();
  });

  it('does not display reload hints for a configured executable or a sent reload', async () => {
    activeGame = { id: 'game-1', game_exe: 'C:/Fixture/Game.exe' };
    runtimeSyncByGame = {
      'game-1': {
        game_id: 'game-1',
        generation: 4,
        phase: 'needs_manual_reload',
        cause: 'effective_mods_changed',
        reload_binding: 'F10',
      },
    };
    const { rerender } = await renderTopBar();
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    runtimeSyncByGame['game-1'] = {
      game_id: 'game-1',
      generation: 5,
      phase: 'succeeded',
      reload_status: 'sent',
    };
    rerender(<TopBar />);
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('opens game setup from Quick Play when standalone launch lacks an executable', async () => {
    activeGame = { id: 'game-1', game_exe: null, launch_mode: 'standalone' };
    await renderTopBar();

    fireEvent.click(screen.getByTitle('App Menu'));
    fireEvent.click(screen.getByRole('button', { name: 'Quick Play' }));

    expect(mockSetSettingsTab).toHaveBeenCalledWith('games');
    expect(mockSetWorkspaceView).toHaveBeenCalledWith('settings');
  });
});
