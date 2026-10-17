import { act, fireEvent, render, screen } from '@testing-library/react';
import type { ComponentProps, ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { GameConfig } from '@/entities/game';
import { DashboardQuickActions } from './DashboardQuickActions';

const launchConfiguredGame = vi.fn();

vi.mock('@/entities/game', () => ({
  launchConfiguredGame: (...args: unknown[]) => launchConfiguredGame(...args),
  requiresGameExecutable: (game: Pick<GameConfig, 'game_exe' | 'launch_mode'>) =>
    game.launch_mode !== 'xxmi_managed' && !game.game_exe?.trim(),
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
  beforeEach(() => {
    launchConfiguredGame.mockReset();
  });

  const activeGame: GameConfig = {
    id: 'game-1',
    name: 'GIMI',
    game_type: 0,
    mod_path: 'E:/GIMI/Mods',
    game_exe: 'E:/GIMI/GenshinImpact.exe',
    loader_exe: null,
    launch_args: null,
  };
  const renderActions = (overrides: Partial<ComponentProps<typeof DashboardQuickActions>> = {}) =>
    render(
      <DashboardQuickActions
        activeGame={activeGame}
        setWorkspaceView={vi.fn()}
        onSetupExecutable={vi.fn()}
        {...overrides}
      />,
    );

  it('opens Mod Inbox from its dashboard tile', () => {
    const setWorkspaceView = vi.fn();
    renderActions({ setWorkspaceView });

    fireEvent.click(screen.getByRole('button', { name: 'Mod Inbox' }));

    expect(setWorkspaceView).toHaveBeenCalledWith('mod-inbox');
  });

  it('keeps the dashboard actions in the shared task order', () => {
    renderActions();

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
    renderActions();

    const quickPlay = screen.getByRole('button', { name: 'Quick Play' });
    fireEvent.click(quickPlay);

    expect(launchConfiguredGame).toHaveBeenCalledWith('game-1', true);
    expect(quickPlay).toBeDisabled();

    await act(async () => resolveLaunch());
  });

  it('routes standalone Play without an executable to its setup', () => {
    const onSetupExecutable = vi.fn();
    renderActions({ activeGame: { ...activeGame, game_exe: null }, onSetupExecutable });

    fireEvent.click(screen.getByRole('button', { name: 'Quick Play' }));

    expect(onSetupExecutable).toHaveBeenCalledOnce();
    expect(launchConfiguredGame).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Set up executable' })).toBeInTheDocument();
  });

  it('allows XXMI Play without an optional game executable', async () => {
    renderActions({ activeGame: { ...activeGame, game_exe: null, launch_mode: 'xxmi_managed' } });

    fireEvent.click(screen.getByRole('button', { name: 'Quick Play' }));

    expect(launchConfiguredGame).toHaveBeenCalledWith('game-1', true);
  });
});
