import { act, renderHook, waitFor } from '@testing-library/react';
import { listen } from '@tauri-apps/api/event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useDiskReconcileProgress } from './reconcileProgress';

const setDiskReconcileProgress = vi.fn();
const markDiskReconcilePending = vi.fn();

vi.mock('../../../app/store/useAppStore', () => ({
  useAppStore: (
    selector: (state: {
      setDiskReconcileProgress: typeof setDiskReconcileProgress;
      markDiskReconcilePending: typeof markDiskReconcilePending;
    }) => unknown,
  ) => selector({ setDiskReconcileProgress, markDiskReconcilePending }),
}));

describe('useDiskReconcileProgress', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('clears failed progress instead of leaving the workspace syncing', async () => {
    let handler: ((event: { payload: Record<string, unknown> }) => void) | undefined;
    vi.mocked(listen).mockImplementation(async (_event, callback) => {
      handler = callback as unknown as (event: { payload: Record<string, unknown> }) => void;
      return () => undefined;
    });

    renderHook(() => useDiskReconcileProgress('game-1'));
    await waitFor(() => expect(handler).toBeDefined());

    act(() => {
      handler?.({
        payload: {
          game_id: 'game-1',
          phase: 'Failed',
        },
      });
    });

    expect(setDiskReconcileProgress).toHaveBeenCalledWith('game-1', null);
    expect(markDiskReconcilePending).toHaveBeenCalledWith('game-1', true);
  });

  it('clears progress when a rename-confirmation reconcile reaches its terminal completion phase', async () => {
    let handler: ((event: { payload: Record<string, unknown> }) => void) | undefined;
    vi.mocked(listen).mockImplementation(async (_event, callback) => {
      handler = callback as unknown as (event: { payload: Record<string, unknown> }) => void;
      return () => undefined;
    });

    renderHook(() => useDiskReconcileProgress('game-1'));
    await waitFor(() => expect(handler).toBeDefined());

    act(() => {
      handler?.({
        payload: {
          game_id: 'game-1',
          run_id: 'rename-confirmation-run',
          reason: 'WatcherBatch',
          phase: 'Completed',
          completed_units: 1,
          total_units: 1,
          current_root: null,
          elapsed_ms: 42,
          eta_ms: null,
        },
      });
    });

    expect(setDiskReconcileProgress).toHaveBeenCalledWith('game-1', null);
    expect(markDiskReconcilePending).not.toHaveBeenCalled();
  });
});
