import { beforeEach, describe, expect, it, vi } from 'vitest';
import { launchConfiguredGame, requiresGameExecutable } from './launchConfiguredGame';
import { commands } from '@/shared/api/tauri/bindings';

vi.mock('@/shared/api/tauri/bindings', () => ({
  commands: {
    launchGame: vi.fn(),
    exitApp: vi.fn(),
  },
}));

describe('requiresGameExecutable', () => {
  it.each([null, '', '   '])('requires an executable for standalone launch: %s', (game_exe) => {
    expect(requiresGameExecutable({ launch_mode: 'standalone', game_exe })).toBe(true);
    expect(requiresGameExecutable({ game_exe })).toBe(true);
  });

  it('accepts a configured standalone executable', () => {
    expect(
      requiresGameExecutable({ launch_mode: 'standalone', game_exe: 'C:/Games/Game.exe' }),
    ).toBe(false);
  });

  it('allows XXMI-managed launch without a game executable', () => {
    expect(requiresGameExecutable({ launch_mode: 'xxmi_managed', game_exe: null })).toBe(false);
  });
});

describe('launchConfiguredGame', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('exits only after a successful launch when auto-close is enabled', async () => {
    await launchConfiguredGame('game-1', true);

    expect(commands.launchGame).toHaveBeenCalledWith('game-1');
    expect(commands.exitApp).toHaveBeenCalledOnce();
  });

  it('does not exit when launching fails', async () => {
    vi.mocked(commands.launchGame).mockRejectedValueOnce(new Error('Launch failed'));

    await expect(launchConfiguredGame('game-1', true)).rejects.toThrow('Launch failed');

    expect(commands.exitApp).not.toHaveBeenCalled();
  });
});
