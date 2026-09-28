import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/app/store';
import { waitForGameActivationReady } from './waitForGameActivationReady';

describe('waitForGameActivationReady', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useAppStore.setState({ gameActivationByGame: {} });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('waits for disk-authoritative activation before leaving onboarding', async () => {
    const ready = waitForGameActivationReady('game-1');
    let completed = false;
    void ready.then(() => {
      completed = true;
    });

    useAppStore.getState().setGameActivationStatus({
      game_id: 'game-1',
      generation: 1,
      phase: 'syncing',
      reconcile_revision: null,
      runtime_sync_generation: null,
      error: null,
    });
    await Promise.resolve();
    expect(completed).toBe(false);

    useAppStore.getState().setGameActivationStatus({
      game_id: 'game-1',
      generation: 1,
      phase: 'ready',
      reconcile_revision: 7,
      runtime_sync_generation: null,
      error: null,
    });
    await ready;
    expect(completed).toBe(true);
  });

  it('rejects an activation failure instead of opening a locked workspace', async () => {
    const ready = waitForGameActivationReady('game-2');
    useAppStore.getState().setGameActivationStatus({
      game_id: 'game-2',
      generation: 1,
      phase: 'failed',
      reconcile_revision: null,
      runtime_sync_generation: null,
      error: 'Watcher unavailable',
    });
    await expect(ready).rejects.toThrow('Watcher unavailable');
  });

  it('times out if activation never publishes a terminal status', async () => {
    const ready = waitForGameActivationReady('game-3');
    const rejection = expect(ready).rejects.toThrow('Timed out waiting for game');
    await vi.runAllTimersAsync();
    await rejection;
  });
});
