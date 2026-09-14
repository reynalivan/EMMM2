import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createWrapper } from '../../../tests/testing/test-utils';
import { useStorageSizeBackfill } from './useStorageSizeBackfill';
import { commands } from '@/shared/api/tauri/bindings.gen';
import { publishQueryScopes } from '@/shared/lib/queryRefresh';

vi.mock('@tanstack/react-query', async () => await vi.importActual('@tanstack/react-query'));

vi.mock('@/shared/api/tauri/bindings.gen', () => ({
  commands: {
    startStorageSizeBackfill: vi.fn(),
    getStorageSizeBackfillStatus: vi.fn(),
  },
}));

vi.mock('@/shared/lib/queryRefresh', () => ({
  publishQueryScopes: vi.fn().mockResolvedValue(undefined),
}));

const runningStatus = {
  state: 'Running' as const,
  total_games: 2,
  completed_games: 1,
  current_game_id: 'g-2',
  errors: [],
};

describe('useStorageSizeBackfill', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  it('starts a backfill and refreshes dashboard statistics after completion', async () => {
    vi.mocked(commands.startStorageSizeBackfill).mockResolvedValue({
      status: 'ok',
      data: runningStatus,
    });
    vi.mocked(commands.getStorageSizeBackfillStatus).mockResolvedValue({
      ...runningStatus,
      state: 'Completed',
      completed_games: 2,
      current_game_id: null,
    });

    const { result } = renderHook(() => useStorageSizeBackfill(), { wrapper: createWrapper });

    await act(async () => {
      await Promise.resolve();
    });
    expect(commands.startStorageSizeBackfill).toHaveBeenCalledTimes(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });

    expect(result.current.status).toMatchObject({ state: 'Completed', completed_games: 2 });
    expect(publishQueryScopes).toHaveBeenCalledWith(expect.anything(), ['dashboard']);
  });

  it('stops polling after the dashboard unmounts', async () => {
    vi.mocked(commands.startStorageSizeBackfill).mockResolvedValue({
      status: 'ok',
      data: runningStatus,
    });

    const { unmount } = renderHook(() => useStorageSizeBackfill(), { wrapper: createWrapper });

    await act(async () => {
      await Promise.resolve();
    });
    expect(commands.startStorageSizeBackfill).toHaveBeenCalledTimes(1);
    const getStatus = vi.mocked(commands.getStorageSizeBackfillStatus);
    const statusCallsBeforeUnmount = getStatus.mock.calls.length;
    unmount();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });

    expect(commands.getStorageSizeBackfillStatus).toHaveBeenCalledTimes(statusCallsBeforeUnmount);
  });

  it('restarts the backfill after a failed start', async () => {
    vi.mocked(commands.startStorageSizeBackfill)
      .mockResolvedValueOnce({ status: 'error', error: { type: 'Io', payload: 'Scan failed' } })
      .mockResolvedValueOnce({
        status: 'ok',
        data: {
          state: 'Completed',
          total_games: 2,
          completed_games: 2,
          current_game_id: null,
          errors: [],
        },
      });

    const { result } = renderHook(() => useStorageSizeBackfill(), { wrapper: createWrapper });

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(result.current.status).toMatchObject({ state: 'Failed', errors: ['Scan failed'] });

    act(() => {
      result.current.retry();
    });

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(commands.startStorageSizeBackfill).toHaveBeenCalledTimes(2);
    expect(result.current.status).toMatchObject({ state: 'Completed', completed_games: 2 });
  });
});
