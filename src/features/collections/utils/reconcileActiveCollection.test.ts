import { describe, it, expect, vi, beforeEach } from 'vitest';
import { reconcileActiveCollection } from './reconcileActiveCollection';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('reconcileActiveCollection', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns false when no active game is selected', async () => {
    const invokeFn = vi.fn();

    const changed = await reconcileActiveCollection(
      {},
      {
        getState: () => ({
          activeGameId: null,
          safeMode: true,
        }),
        invokeFn,
      },
    );

    expect(changed).toBe(false);
    expect(invokeFn).not.toHaveBeenCalled();
  });

  it('returns false when reconcile reports no changes', async () => {
    const invokeFn = vi.fn().mockResolvedValueOnce(0);

    const changed = await reconcileActiveCollection(
      {},
      {
        getState: () => ({
          activeGameId: 'g1',
          safeMode: true,
        }),
        invokeFn,
      },
    );

    expect(changed).toBe(false);
  });

  it('returns true when reconcile changes corridor state', async () => {
    const invokeFn = vi.fn().mockResolvedValueOnce(2);

    const changed = await reconcileActiveCollection(
      {},
      {
        getState: () => ({
          activeGameId: 'g1',
          safeMode: true,
        }),
        invokeFn,
      },
    );

    expect(changed).toBe(true);
  });
});
