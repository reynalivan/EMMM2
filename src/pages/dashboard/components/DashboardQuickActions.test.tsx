import { fireEvent, render, screen } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { DashboardQuickActions } from './DashboardQuickActions';

const launchConfiguredGame = vi.fn();

vi.mock('@/entities/game', () => ({
  launchConfiguredGame: (...args: unknown[]) => launchConfiguredGame(...args),
}));
vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: { autoCloseLauncher: boolean }) => unknown) =>
    selector({ autoCloseLauncher: true }),
}));
vi.mock('@/shared/ui/toast', () => ({ toast: { error: vi.fn() } }));
vi.mock('@/shared/ui/liquid', () => ({
  LiquidSurface: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

describe('DashboardQuickActions', () => {
  it('opens Mod Inbox from its dashboard tile', () => {
    const setWorkspaceView = vi.fn();
    render(<DashboardQuickActions activeGameId="game-1" setWorkspaceView={setWorkspaceView} />);

    fireEvent.click(screen.getByRole('button', { name: 'Mod Inbox' }));

    expect(setWorkspaceView).toHaveBeenCalledWith('mod-inbox');
  });

  it('keeps the dashboard actions in the shared task order', () => {
    render(<DashboardQuickActions activeGameId="game-1" setWorkspaceView={vi.fn()} />);

    expect(screen.getAllByRole('button').map((button) => button.textContent)).toEqual([
      'Quick Play',
      'Mods Manager',
      'Mod Inbox',
      'Collections',
      'Storage Optimizer',
      'Discover',
      'Downloads',
      'Settings',
    ]);
  });

  it('uses the shared launch action and prevents duplicate clicks while pending', async () => {
    let resolveLaunch!: () => void;
    launchConfiguredGame.mockReturnValueOnce(
      new Promise<void>((resolve) => {
        resolveLaunch = resolve;
      }),
    );
    render(<DashboardQuickActions activeGameId="game-1" setWorkspaceView={vi.fn()} />);

    const quickPlay = screen.getByRole('button', { name: 'Quick Play' });
    fireEvent.click(quickPlay);

    expect(launchConfiguredGame).toHaveBeenCalledWith('game-1', true);
    expect(quickPlay).toBeDisabled();

    resolveLaunch();
  });
});
