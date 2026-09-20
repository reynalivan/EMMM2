import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TopBar } from './index';

const mockSetWorkspaceView = vi.fn();
const mockSetAppMenuOpen = vi.fn();
let workspaceView = 'dashboard';
let activeGame: { id: string } | null = null;
let runtimeSyncByGame: Record<string, Record<string, unknown>> = {};
const { mockRetryRuntimeSync } = vi.hoisted(() => ({ mockRetryRuntimeSync: vi.fn() }));

vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      workspaceView,
      setWorkspaceView: mockSetWorkspaceView,
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
}));

vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: { launchGame: vi.fn(), retryRuntimeSync: mockRetryRuntimeSync },
}));
vi.mock('./GameSelector', () => ({ default: () => null }));
vi.mock('./ContextControls', () => ({ default: () => null }));
vi.mock('./GlobalActions', () => ({ default: () => null }));
vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({
    children,
    className,
    ...props
  }: {
    children: ReactNode;
    className?: string;
  }) => (
    <div className={className} {...props}>
      {children}
    </div>
  ),
}));

describe('TopBar app menu', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    workspaceView = 'dashboard';
    activeGame = null;
    runtimeSyncByGame = {};
  });

  it('navigates to Mod Inbox from the app menu', () => {
    render(<TopBar />);

    fireEvent.click(screen.getByTitle('App Menu'));
    fireEvent.click(screen.getByTestId('nav-mod-inbox'));

    expect(mockSetWorkspaceView).toHaveBeenCalledWith('mod-inbox');
  });

  it('includes Discover and Downloads in the dashboard action order', () => {
    render(<TopBar />);

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

  it('keeps the back action inside the app menu', () => {
    workspaceView = 'mods';
    render(<TopBar />);

    expect(screen.queryByTitle('Back to Dashboard')).not.toBeInTheDocument();

    fireEvent.click(screen.getByTitle('App Menu'));

    expect(screen.getByTestId('app-menu-overlay')).toHaveClass('app-menu-overlay');
    fireEvent.click(screen.getByTestId('nav-dashboard'));

    expect(mockSetWorkspaceView).toHaveBeenCalledWith('dashboard');
  });

  it('renders the app menu outside the topbar clipping surface', () => {
    render(<TopBar />);

    fireEvent.click(screen.getByTitle('App Menu'));

    const overlay = screen.getByTestId('app-menu-overlay');
    expect(overlay.parentElement?.parentElement).toBe(document.body);
    expect(overlay.parentElement).toHaveClass('fixed');
  });

  it('publishes the app-menu state while the menu is open', () => {
    render(<TopBar />);

    fireEvent.click(screen.getByTitle('App Menu'));
    expect(mockSetAppMenuOpen).toHaveBeenCalledWith(true);

    fireEvent.click(screen.getByTitle('App Menu'));
    expect(mockSetAppMenuOpen).toHaveBeenCalledWith(false);
  });

  it('shows collection controls only in Mods Manager', () => {
    const contextControls = <span>Collection controls</span>;
    const { rerender } = render(<TopBar contextControls={contextControls} />);

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

  it('uses the top bar title for Discover', () => {
    workspaceView = 'browser';
    render(<TopBar />);

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
    render(<TopBar />);

    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));

    await waitFor(() => expect(mockRetryRuntimeSync).toHaveBeenCalledWith('game-1'));
    expect(screen.getByTestId('topbar-center')).toBeInTheDocument();
  });
});
