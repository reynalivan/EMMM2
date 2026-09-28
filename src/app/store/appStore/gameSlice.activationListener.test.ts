import { expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from '../useAppStore';

it('does not start onboarding activation without its readiness listener', async () => {
  vi.mocked(listen).mockImplementation(async (event) => {
    if (event === 'game_activation:status') {
      throw new Error('Activation listener unavailable');
    }
    return () => undefined;
  });

  await expect(
    useAppStore.getState().setActiveGameId('game-1', {
      requireActivationStatusListener: true,
    }),
  ).rejects.toThrow('Activation listener unavailable');
  expect(invoke).not.toHaveBeenCalledWith('set_active_game', expect.anything());
});
