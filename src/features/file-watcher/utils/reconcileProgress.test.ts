import { act, renderHook, waitFor } from '@testing-library/react';
import { listen, type EventCallback, type UnlistenFn } from '@tauri-apps/api/event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DiskReconcileProgress } from '@/shared/api/tauri/bindings';
import { useDiskReconcileProgress } from './reconcileProgress';

const setDiskReconcileProgress = vi.fn();
const markDiskReconcilePending = vi.fn();

vi.mock('@/app/store', () => ({
  useAppStore: (
    selector: (state: {
      setDiskReconcileProgress: typeof setDiskReconcileProgress;
      markDiskReconcilePending: typeof markDiskReconcilePending;
    }) => unknown,
  ) => selector({ setDiskReconcileProgress, markDiskReconcilePending }),
}));

function progress(overrides: Partial<DiskReconcileProgress> = {}): DiskReconcileProgress {
  return {
    game_id: 'game-1',
    run_id: 'run-1',
    reason: 'WatcherBatch',
    phase: 'ScanningRoots',
    completed_units: 8,
    total_units: 10,
    current_root: null,
    elapsed_ms: 42,
    eta_ms: null,
    ...overrides,
  };
}

describe('useDiskReconcileProgress', () => {
  let handler: EventCallback<DiskReconcileProgress>;

  function emit(overrides: Partial<DiskReconcileProgress> = {}) {
    act(() => {
      handler({ event: 'disk_reconcile:progress', id: 1, payload: progress(overrides) });
    });
  }

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(listen).mockImplementation(async (_event, callback) => {
      handler = callback as EventCallback<DiskReconcileProgress>;
      return () => undefined;
    });
  });

  it('clears failed progress instead of leaving the workspace syncing', () => {
    renderHook(() => useDiskReconcileProgress('game-1'));
    emit({ phase: 'Failed' });

    expect(setDiskReconcileProgress).toHaveBeenCalledWith('game-1', null);
    expect(markDiskReconcilePending).toHaveBeenCalledWith('game-1', true);
  });

  it('clears progress when a rename-confirmation reconcile reaches its terminal phase', () => {
    renderHook(() => useDiskReconcileProgress('game-1'));
    emit({ run_id: 'rename-confirmation-run', phase: 'Completed' });

    expect(setDiskReconcileProgress).toHaveBeenCalledWith('game-1', null);
    expect(markDiskReconcilePending).not.toHaveBeenCalled();
  });

  it('ignores a lower parallel callback count within the same run, phase and total', () => {
    renderHook(() => useDiskReconcileProgress('game-1'));
    emit();
    emit({ completed_units: 7, elapsed_ms: 43 });
    emit({ completed_units: 9, elapsed_ms: 44 });

    expect(setDiskReconcileProgress).toHaveBeenCalledTimes(2);
    expect(setDiskReconcileProgress).toHaveBeenLastCalledWith(
      'game-1',
      progress({ completed_units: 9, elapsed_ms: 44 }),
    );
  });

  it('accepts count resets when the phase, total or run changes', () => {
    renderHook(() => useDiskReconcileProgress('game-1'));
    emit();
    emit({ phase: 'Projecting', completed_units: 2 });
    emit({ phase: 'Projecting', completed_units: 1, total_units: 20 });
    emit({ run_id: 'run-2', phase: 'Projecting', completed_units: 0, total_units: 20 });

    expect(setDiskReconcileProgress).toHaveBeenCalledTimes(4);
  });

  it('accepts a legitimate same-ID retry after terminal completion', () => {
    renderHook(() => useDiskReconcileProgress('game-1'));
    emit();
    emit({ phase: 'Completed' });
    emit({ completed_units: 1 });

    expect(setDiskReconcileProgress).toHaveBeenCalledTimes(3);
    expect(setDiskReconcileProgress).toHaveBeenLastCalledWith(
      'game-1',
      progress({ completed_units: 1 }),
    );
  });

  it('ignores callbacks after unmount while registration is still pending', async () => {
    let resolveRegistration!: (unlisten: UnlistenFn) => void;
    const unlisten = vi.fn();
    vi.mocked(listen).mockImplementation((_event, callback) => {
      handler = callback as EventCallback<DiskReconcileProgress>;
      return new Promise((resolve) => {
        resolveRegistration = resolve;
      });
    });

    const { unmount } = renderHook(() => useDiskReconcileProgress('game-1'));
    unmount();
    emit();
    emit({ phase: 'Failed' });
    resolveRegistration(unlisten);

    await waitFor(() => expect(unlisten).toHaveBeenCalledOnce());
    expect(setDiskReconcileProgress).not.toHaveBeenCalled();
    expect(markDiskReconcilePending).not.toHaveBeenCalled();
  });

  it('does not carry the previous game subscription or counter into a new game', () => {
    const { rerender } = renderHook(({ gameId }) => useDiskReconcileProgress(gameId), {
      initialProps: { gameId: 'game-1' },
    });
    emit();
    const previousHandler = handler;
    rerender({ gameId: 'game-2' });

    act(() => {
      previousHandler({ event: 'disk_reconcile:progress', id: 1, payload: progress() });
    });
    emit({ game_id: 'game-2', completed_units: 1 });

    expect(setDiskReconcileProgress).toHaveBeenCalledTimes(2);
    expect(setDiskReconcileProgress).toHaveBeenLastCalledWith(
      'game-2',
      progress({ game_id: 'game-2', completed_units: 1 }),
    );
  });

  it('reports registration failure without leaving an unhandled cleanup rejection', async () => {
    const error = new Error('Native registration failed');
    const log = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    vi.mocked(listen).mockRejectedValueOnce(error);

    try {
      const { unmount } = renderHook(() => useDiskReconcileProgress('game-1'));
      await waitFor(() =>
        expect(log).toHaveBeenCalledWith(
          'Failed to register disk reconcile progress listener',
          error,
        ),
      );
      unmount();
      await act(async () => undefined);
      expect(log).toHaveBeenCalledOnce();
      expect(setDiskReconcileProgress).not.toHaveBeenCalled();
    } finally {
      log.mockRestore();
    }
  });
});
