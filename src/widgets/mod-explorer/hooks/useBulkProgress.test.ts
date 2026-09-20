import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useBulkProgress, type BulkProgressPayload } from './useBulkProgress';

const eventState = vi.hoisted(() => ({
  listener: null as ((event: { payload: BulkProgressPayload }) => void) | null,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(
    async (_event: string, listener: (event: { payload: BulkProgressPayload }) => void) => {
      eventState.listener = listener;
      return vi.fn();
    },
  ),
}));

vi.mock('@/shared/lib/appMode', () => ({ isDemoMode: false }));

describe('useBulkProgress', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    eventState.listener = null;
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('does not let an older completion timer hide a newer operation', async () => {
    const { result } = renderHook(() => useBulkProgress());
    await act(async () => {
      await Promise.resolve();
    });
    expect(eventState.listener).not.toBeNull();

    act(() => {
      eventState.listener?.({
        payload: {
          operation_id: 'toggle-old',
          cancellable: true,
          label: 'old',
          current: 1,
          total: 1,
          active: true,
        },
      });
      eventState.listener?.({
        payload: {
          operation_id: 'toggle-new',
          cancellable: true,
          label: 'new',
          current: 0,
          total: 2,
          active: true,
        },
      });
      vi.advanceTimersByTime(1_500);
    });

    expect(result.current.operation_id).toBe('toggle-new');
    expect(result.current.active).toBe(true);
  });
});
