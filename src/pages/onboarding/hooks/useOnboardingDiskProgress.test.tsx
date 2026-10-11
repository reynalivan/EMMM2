import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { listen, type Event } from '@tauri-apps/api/event';
import type { DiskReconcileProgress } from '@/shared/api/tauri/bindings';
import { useOnboardingDiskProgress } from './useOnboardingDiskProgress';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

function progress(overrides: Partial<DiskReconcileProgress> = {}): DiskReconcileProgress {
  return {
    game_id: 'game-1',
    run_id: 'session-1:game-1',
    reason: 'OnboardingCompleted',
    phase: 'DiscoveringRoots',
    completed_units: 0,
    total_units: null,
    current_root: null,
    elapsed_ms: 0,
    eta_ms: null,
    ...overrides,
  };
}

describe('onboarding progress ownership', () => {
  let handler: ((event: Event<DiskReconcileProgress>) => void) | undefined;

  beforeEach(() => {
    vi.clearAllMocks();
    handler = undefined;
    vi.mocked(listen).mockImplementation(async (_event, callback) => {
      handler = callback as (event: Event<DiskReconcileProgress>) => void;
      return () => undefined;
    });
  });

  it('ignores watcher progress for the same game during onboarding', async () => {
    const { result } = renderHook(() => useOnboardingDiskProgress());
    await act(() => result.current.start('session-1', 'game-1'));
    act(() => {
      handler?.({
        event: 'disk_reconcile:progress',
        id: 1,
        payload: progress({ reason: 'WatcherBatch', phase: 'Completed' }),
      });
    });
    expect(result.current.progress).toBeNull();
  });

  it('accepts only its exact run and keeps phase and counts monotonic until terminal', async () => {
    const { result } = renderHook(() => useOnboardingDiskProgress());
    await act(() => result.current.start('session-1', 'game-1'));
    const emit = (payload: DiskReconcileProgress) => {
      act(() => handler?.({ event: 'disk_reconcile:progress', id: 1, payload }));
    };
    emit(progress({ run_id: 'old-session:game-1' }));
    emit(progress({ game_id: 'game-2' }));
    expect(result.current.progress).toBeNull();
    emit(progress({ phase: 'ScanningRoots', completed_units: 2, total_units: 4, elapsed_ms: 500 }));
    emit(progress({ phase: 'ScanningRoots', completed_units: 1, total_units: 4, elapsed_ms: 600 }));
    expect(result.current.progress?.completed_units).toBe(2);
    emit(progress({ phase: 'Projecting', elapsed_ms: 700 }));
    emit(progress({ phase: 'ScanningRoots', completed_units: 4, total_units: 4, elapsed_ms: 800 }));
    expect(result.current.progress?.phase).toBe('Projecting');
    emit(progress({ phase: 'Completed', elapsed_ms: 900 }));
    emit(progress({ phase: 'Failed', elapsed_ms: 1000 }));
    expect(result.current.progress?.phase).toBe('Completed');
    emit(
      progress({
        run_id: 'session-1:game-1:1',
        phase: 'ScanningRoots',
        completed_units: 0,
        total_units: 4,
      }),
    );
    expect(result.current.progress?.phase).toBe('ScanningRoots');
    emit(progress({ phase: 'Completed', elapsed_ms: 1200 }));
    expect(result.current.progress?.run_id).toBe('session-1:game-1:1');
    emit(progress({ run_id: 'session-1:game-1:1', phase: 'Failed', elapsed_ms: 200 }));
    expect(result.current.progress?.phase).toBe('Failed');
  });

  it('retires old callbacks and unsubscribes when a new attempt starts or unmounts', async () => {
    const unlisten = vi.fn();
    vi.mocked(listen).mockImplementation(async (_event, callback) => {
      handler = callback as (event: Event<DiskReconcileProgress>) => void;
      return unlisten;
    });
    const { result, unmount } = renderHook(() => useOnboardingDiskProgress());
    await act(() => result.current.start('session-1', 'game-1'));
    const oldHandler = handler;
    await act(() => result.current.start('session-2', 'game-1'));
    expect(unlisten).toHaveBeenCalledTimes(1);
    act(() => oldHandler?.({ event: 'disk_reconcile:progress', id: 1, payload: progress() }));
    expect(result.current.progress).toBeNull();
    act(() =>
      handler?.({
        event: 'disk_reconcile:progress',
        id: 1,
        payload: progress({ run_id: 'session-2:game-1' }),
      }),
    );
    expect(result.current.progress?.run_id).toBe('session-2:game-1');
    unmount();
    expect(unlisten).toHaveBeenCalledTimes(2);
  });

  it('releases a pending registration after unmount and rejects its start barrier', async () => {
    let finishRegistration: ((unlisten: () => void) => void) | undefined;
    vi.mocked(listen).mockReturnValue(
      new Promise((resolve) => {
        finishRegistration = resolve;
      }),
    );
    const unlisten = vi.fn();
    const { result, unmount } = renderHook(() => useOnboardingDiskProgress());
    const start = result.current.start('session-1', 'game-1');
    const rejection = expect(start).rejects.toThrow('subscription stopped');
    unmount();
    finishRegistration?.(unlisten);
    await rejection;
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('propagates registration errors so indexing cannot proceed without progress ownership', async () => {
    vi.mocked(listen).mockRejectedValue(new Error('Listener unavailable'));
    const { result } = renderHook(() => useOnboardingDiskProgress());
    await expect(result.current.start('session-1', 'game-1')).rejects.toThrow(
      'Listener unavailable',
    );
    expect(result.current.progress).toBeNull();
  });
});
