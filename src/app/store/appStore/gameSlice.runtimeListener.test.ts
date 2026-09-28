import { expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from '../useAppStore';

it('allows onboarding activation when only the optional runtime listener fails', async () => {
  vi.mocked(listen).mockImplementation(async (event) => {
    if (event === 'runtime_sync:status') {
      throw new Error('Runtime listener unavailable');
    }
    return () => undefined;
  });
  vi.mocked(invoke).mockResolvedValue({
    game_id: 'game-1',
    generation: 1,
    phase: 'syncing',
  });

  await expect(
    useAppStore.getState().setActiveGameId('game-1', {
      deferWorkspacePrefetch: true,
      requireActivationStatusListener: true,
    }),
  ).resolves.toBeUndefined();
  expect(invoke).toHaveBeenCalledWith('set_active_game', expect.anything());
});
