import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { describe, expect, it, vi } from 'vitest';

describe('active game requests during listener registration', () => {
  it('submits only the latest game after listeners become available', async () => {
    let releaseListeners!: () => void;
    const listenersReady = new Promise<() => void>((resolve) => {
      releaseListeners = () => resolve(() => undefined);
    });
    vi.mocked(listen).mockImplementation((event) =>
      event === 'runtime_sync:status' || event === 'game_activation:status'
        ? listenersReady
        : Promise.resolve(() => undefined),
    );
    vi.mocked(invoke).mockImplementation((command, args) => {
      if (command === 'set_active_game') {
        return Promise.resolve({
          game_id: (args as { gameId: string }).gameId,
          generation: 1,
          phase: 'syncing',
        });
      }
      return Promise.reject(new Error(`Unexpected command: ${command}`));
    });

    const { useAppStore } = await import('./useAppStore');
    const first = useAppStore.getState().setActiveGameId('game-b', {
      deferWorkspacePrefetch: true,
    });
    const latest = useAppStore.getState().setActiveGameId('game-c', {
      deferWorkspacePrefetch: true,
    });
    expect(useAppStore.getState().requestedGameId).toBe('game-c');

    releaseListeners();
    await Promise.all([first, latest]);

    const activationCalls = vi
      .mocked(invoke)
      .mock.calls.filter(([command]) => command === 'set_active_game');
    expect(activationCalls).toHaveLength(1);
    expect(activationCalls[0]?.[1]).toEqual({ gameId: 'game-c' });
    expect(useAppStore.getState().activeGameId).toBe('game-c');
  });
});
