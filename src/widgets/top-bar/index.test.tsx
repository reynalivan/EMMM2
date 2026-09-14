import { fireEvent, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TopBar } from './index';

const mockSetWorkspaceView = vi.fn();
let workspaceView = 'dashboard';

vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      workspaceView,
      setWorkspaceView: mockSetWorkspaceView,
      safetyFilter: 'all',
      setSafetyFilter: vi.fn(),
      autoCloseLauncher: false,
    }),
}));

vi.mock('@/entities/game', () => ({
  useActiveGame: () => ({ activeGame: null }),
}));

vi.mock('@/shared/api/tauri/bindings', () => ({ commands: { launchGame: vi.fn() } }));
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
      'nav-storage-optimizer',
      'nav-collections',
      'nav-settings',
      'nav-browser',
      'nav-downloads',
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

  it('uses the top bar title for Browser', () => {
    workspaceView = 'browser';
    render(<TopBar />);

    expect(screen.getByText('Browser')).toBeInTheDocument();
  });
});
