import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useGameSwitch } from './useGameSwitch';

const { state, publishRuntimeDescriptor } = vi.hoisted(() => ({
  state: {
    activeGameId: 'game-a' as string | null,
    requestedGameId: null as string | null,
    setActiveGameId: vi.fn<(_gameId: string) => Promise<void>>(),
  },
  publishRuntimeDescriptor: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@/app/store', () => ({
  useAppStore: Object.assign((selector: (value: typeof state) => unknown) => selector(state), {
    getState: () => state,
  }),
}));
vi.mock('@tanstack/react-query', () => ({ useQueryClient: () => ({}) }));
vi.mock('@/shared/lib/queryRefresh', () => ({ publishRuntimeDescriptor }));
vi.mock('./objectMutationCache', () => ({ buildObjectListRefreshDescriptor: () => ({}) }));

describe('useGameSwitch', () => {
  beforeEach(() => {
    state.activeGameId = 'game-a';
    state.requestedGameId = null;
    state.setActiveGameId.mockReset();
    publishRuntimeDescriptor.mockClear();
  });

  it('does not refresh the workspace for a switch superseded by another game', async () => {
    let resolveFirst!: () => void;
    state.setActiveGameId.mockImplementation((gameId) => {
      state.requestedGameId = gameId;
      if (gameId === 'game-a') {
        return new Promise<void>((resolve) => {
          resolveFirst = resolve;
        });
      }
      state.activeGameId = gameId;
      state.requestedGameId = null;
      return Promise.resolve();
    });
    const { result } = renderHook(() => useGameSwitch());

    let first!: Promise<void>;
    let second!: Promise<void>;
    act(() => {
      first = result.current.switchGame('game-a');
      second = result.current.switchGame('game-b');
    });
    await act(async () => {
      await second;
    });
    resolveFirst();
    await act(async () => {
      await first;
    });

    expect(publishRuntimeDescriptor).toHaveBeenCalledTimes(1);
  });
});
