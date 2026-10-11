import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useDeleteObject } from './useObjectMutations';

vi.mock('@tanstack/react-query', async () =>
  vi.importActual<typeof import('@tanstack/react-query')>('@tanstack/react-query'),
);

const mocks = vi.hoisted(() => ({
  deleteObject: vi.fn(),
  openRetry: vi.fn(),
  publish: vi.fn(),
  gameId: 'game',
}));
vi.mock('@/entities/game-object', () => ({
  deleteObject: mocks.deleteObject,
  createObject: vi.fn(),
  updateObject: vi.fn(),
}));
vi.mock('./fileInUseRetry', () => ({ openFileInUseRetryDialog: mocks.openRetry }));
vi.mock('@/shared/lib/queryRefresh', () => ({ publishRuntimeDescriptor: mocks.publish }));
vi.mock('@/app/store', () => ({
  useAppStore: { getState: () => ({ activeGameId: mocks.gameId }) },
}));

function wrapper({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { mutations: { retry: false } } })}
    >
      {children}
    </QueryClientProvider>
  );
}

describe('object deletion lock recovery', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.gameId = 'game';
  });

  it('provides an awaitable retry with the original object and recovery capability', async () => {
    const error = { type: 'FileLock', payload: { path: 'C:/Mods/Object', recovery_id: 'token' } };
    mocks.deleteObject.mockRejectedValueOnce(error).mockResolvedValue(undefined);
    const { result } = renderHook(() => useDeleteObject(), { wrapper });
    await act(async () => {
      await expect(result.current.mutateAsync({ id: 'object', force: true })).rejects.toEqual(
        error,
      );
    });
    const retry = mocks.openRetry.mock.calls[0][2];
    await act(async () => {
      await retry({ id: 'object', force: true, recoveryId: 'token' });
    });
    expect(mocks.deleteObject).toHaveBeenLastCalledWith('object', true, 'token');
    expect(mocks.publish).toHaveBeenCalledTimes(1);
  });

  it('does not open a recovery dialog for a superseded game', async () => {
    let reject!: (error: unknown) => void;
    mocks.deleteObject.mockImplementation(
      () =>
        new Promise<void>((_, fail) => {
          reject = fail;
        }),
    );
    const { result } = renderHook(() => useDeleteObject(), { wrapper });
    let mutation!: Promise<void>;
    await act(async () => {
      mutation = result.current.mutateAsync({ id: 'object', force: true });
      await Promise.resolve();
    });
    mocks.gameId = 'other';
    await act(async () => {
      reject({ type: 'FileLock', payload: { path: 'C:/Mods/Object', recovery_id: 'token' } });
      await expect(mutation).rejects.toBeDefined();
    });
    expect(mocks.openRetry).not.toHaveBeenCalled();
  });
});
