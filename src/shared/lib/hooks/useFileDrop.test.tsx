import { act, renderHook, waitFor } from '@testing-library/react';
import type { EventCallback, UnlistenFn } from '@tauri-apps/api/event';
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from 'vitest';
import { useFileDrop } from './useFileDrop';

interface DropPayload {
  paths: string[];
  position: { x: number; y: number };
}

const mocks = vi.hoisted(() => ({ listen: vi.fn(), report: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: mocks.listen }));
vi.mock('@/shared/lib/appMode', () => ({ isDemoMode: false }));
vi.mock('@/shared/lib/telemetry', () => ({ reportFrontendError: mocks.report }));

describe('useFileDrop listener ownership', () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => vi.unstubAllGlobals());

  it.each(['unmount', 'disable'] as const)(
    'disposes registrations that finish after %s and ignores their events',
    async (cleanup) => {
      const pending: Array<{
        event: string;
        callback: EventCallback<DropPayload>;
        resolve: (unlisten: UnlistenFn) => void;
        unlisten: Mock<UnlistenFn>;
      }> = [];
      mocks.listen.mockImplementation(
        (event: string, callback: EventCallback<DropPayload>) =>
          new Promise<UnlistenFn>((resolve) => {
            pending.push({ event, callback, resolve, unlisten: vi.fn<UnlistenFn>() });
          }),
      );
      const onDrop = vi.fn();
      const onDragStateChange = vi.fn();
      const hook = renderHook(
        ({ enabled }) => useFileDrop({ enabled, onDrop, onDragStateChange }),
        { initialProps: { enabled: true } },
      );
      if (cleanup === 'unmount') hook.unmount();
      else hook.rerender({ enabled: false });

      for (let index = 0; index < 4; index += 1) {
        await waitFor(() => expect(pending.length).toBeGreaterThan(index));
        await act(async () => pending[index].resolve(pending[index].unlisten));
      }
      for (const listener of pending) {
        expect(listener.unlisten).toHaveBeenCalledOnce();
        listener.callback({
          event: listener.event,
          id: 1,
          payload: { paths: ['D:/Incoming/Mod'], position: { x: 10, y: 20 } },
        });
      }
      expect(onDrop).not.toHaveBeenCalled();
      expect(onDragStateChange).not.toHaveBeenCalled();
    },
  );

  it('cleans up every registration when one listener fails', async () => {
    const unlisten = vi.fn();
    const error = new Error('Listener registration failed');
    mocks.listen.mockImplementation((event: string) =>
      event === 'tauri://drag-over' ? Promise.reject(error) : Promise.resolve(unlisten),
    );
    const { unmount } = renderHook(() => useFileDrop({ onDrop: vi.fn() }));
    await waitFor(() =>
      expect(mocks.report).toHaveBeenCalledWith(error, 'unhandled_rejection', { promptUser: true }),
    );
    expect(unlisten).toHaveBeenCalledTimes(3);
    unmount();
    expect(unlisten).toHaveBeenCalledTimes(3);
  });

  it('delivers one live drop with logical coordinates and unregisters on cleanup', async () => {
    const handlers = new Map<string, EventCallback<DropPayload>>();
    const unlisteners: Array<Mock<UnlistenFn>> = [];
    mocks.listen.mockImplementation((event: string, callback: EventCallback<DropPayload>) => {
      handlers.set(event, callback);
      const unlisten = vi.fn<UnlistenFn>();
      unlisteners.push(unlisten);
      return Promise.resolve(unlisten);
    });
    vi.stubGlobal('devicePixelRatio', 2);
    const onDrop = vi.fn();
    const { unmount } = renderHook(() => useFileDrop({ onDrop }));
    await waitFor(() => expect(handlers.has('tauri://drag-drop')).toBe(true));
    act(() =>
      handlers.get('tauri://drag-drop')?.({
        event: 'tauri://drag-drop',
        id: 1,
        payload: { paths: ['D:/Incoming/Mod'], position: { x: 200, y: 100 } },
      }),
    );
    expect(onDrop).toHaveBeenCalledExactlyOnceWith(['D:/Incoming/Mod'], { x: 100, y: 50 });
    unmount();
    for (const unlisten of unlisteners) expect(unlisten).toHaveBeenCalledOnce();
  });
});
