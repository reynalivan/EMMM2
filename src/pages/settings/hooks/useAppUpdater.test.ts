import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { AppUpdateInfo, AppUpdateProgress } from '../../../shared/api/tauri/bindings.gen';

interface ProgressChannel {
  onmessage: ((event: AppUpdateProgress) => void) | null;
}

const { checkAppUpdate, installAppUpdate } = vi.hoisted(() => ({
  checkAppUpdate: vi.fn<() => Promise<AppUpdateInfo | null>>(),
  installAppUpdate: vi.fn<(channel: ProgressChannel) => Promise<void>>(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  Channel: class {
    onmessage: ((event: AppUpdateProgress) => void) | null = null;
  },
}));

vi.mock('../../../shared/api/tauri/bindings', () => ({
  commands: {
    checkAppUpdate,
    installAppUpdate,
  },
}));

const AVAILABLE_UPDATE: AppUpdateInfo = {
  version: '1.1.0',
  currentVersion: '1.0.0',
  body: null,
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((onResolve, onReject) => {
    resolve = onResolve;
    reject = onReject;
  });
  return { promise, resolve, reject };
}

let useAppUpdater: typeof import('./useAppUpdater').useAppUpdater;

describe('useAppUpdater', () => {
  beforeEach(async () => {
    vi.resetModules();
    checkAppUpdate.mockReset();
    installAppUpdate.mockReset();
    useAppUpdater = (await import('./useAppUpdater')).useAppUpdater;
  });

  it('checks for update successfully', async () => {
    checkAppUpdate.mockResolvedValue(AVAILABLE_UPDATE);

    const { result } = renderHook(() => useAppUpdater());

    await act(async () => {
      await result.current.checkForUpdate();
    });

    expect(result.current.isChecking).toBe(false);
    expect(result.current.update).toEqual(AVAILABLE_UPDATE);
    expect(result.current.error).toBe(null);
    expect(result.current.hasChecked).toBe(true);
  });

  it('releases a failed check so it can be retried', async () => {
    checkAppUpdate
      .mockRejectedValueOnce(new Error('Network error'))
      .mockResolvedValueOnce(AVAILABLE_UPDATE);

    const { result } = renderHook(() => useAppUpdater());

    await act(async () => {
      await result.current.checkForUpdate();
    });

    expect(result.current.isChecking).toBe(false);
    expect(result.current.update).toBe(null);
    expect(result.current.error).toContain('Network error');

    await act(async () => {
      await result.current.checkForUpdate();
    });

    expect(checkAppUpdate).toHaveBeenCalledTimes(2);
    expect(result.current.update).toEqual(AVAILABLE_UPDATE);
    expect(result.current.isChecking).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('allows dismissing an idle update', async () => {
    checkAppUpdate.mockResolvedValue(AVAILABLE_UPDATE);
    const { result } = renderHook(() => useAppUpdater());

    await act(async () => {
      await result.current.checkForUpdate();
    });
    act(() => {
      result.current.dismiss();
    });

    expect(result.current.update).toBeNull();
    expect(result.current.error).toBeNull();
    expect(result.current.progress).toBeNull();
    expect(result.current.hasChecked).toBe(false);
  });

  it('preserves a pending check across remounts and rejects duplicate actions', async () => {
    const checking = deferred<AppUpdateInfo | null>();
    checkAppUpdate.mockResolvedValueOnce(AVAILABLE_UPDATE).mockReturnValueOnce(checking.promise);
    const first = renderHook(() => useAppUpdater());
    await act(async () => {
      await first.result.current.checkForUpdate();
    });

    let completion!: Promise<void>;
    act(() => {
      completion = first.result.current.checkForUpdate();
      void first.result.current.checkForUpdate();
      void first.result.current.downloadAndInstall();
    });
    first.unmount();
    const second = renderHook(() => useAppUpdater());

    expect(second.result.current.isChecking).toBe(true);
    expect(second.result.current.update).toEqual(AVAILABLE_UPDATE);
    await act(async () => {
      await second.result.current.checkForUpdate();
      await second.result.current.downloadAndInstall();
      second.result.current.dismiss();
    });

    expect(checkAppUpdate).toHaveBeenCalledTimes(2);
    expect(installAppUpdate).not.toHaveBeenCalled();
    expect(second.result.current.isChecking).toBe(true);
    expect(second.result.current.update).toEqual(AVAILABLE_UPDATE);

    await act(async () => {
      checking.resolve(null);
      await completion;
    });

    expect(second.result.current.isChecking).toBe(false);
    expect(second.result.current.update).toBeNull();
    expect(second.result.current.hasChecked).toBe(true);
  });

  it('preserves an install and detached progress across remounts without admitting another job', async () => {
    const installing = deferred<void>();
    checkAppUpdate.mockResolvedValue(AVAILABLE_UPDATE);
    installAppUpdate.mockReturnValue(installing.promise);
    const first = renderHook(() => useAppUpdater());
    await act(async () => {
      await first.result.current.checkForUpdate();
    });

    let completion!: Promise<void>;
    act(() => {
      completion = first.result.current.downloadAndInstall();
      void first.result.current.downloadAndInstall();
    });
    expect(installAppUpdate).toHaveBeenCalledTimes(1);
    const channel = installAppUpdate.mock.calls[0][0];
    act(() => {
      channel.onmessage?.({ event: 'Started', data: { contentLength: 100 } });
      channel.onmessage?.({ event: 'Progress', data: { chunkLength: 20 } });
    });
    first.unmount();
    channel.onmessage?.({ event: 'Progress', data: { chunkLength: 30 } });
    const second = renderHook(() => useAppUpdater());

    expect(second.result.current.isInstalling).toBe(true);
    expect(second.result.current.update).toEqual(AVAILABLE_UPDATE);
    expect(second.result.current.progress).toEqual({ downloaded: 50, total: 100 });
    await act(async () => {
      await second.result.current.downloadAndInstall();
      await second.result.current.checkForUpdate();
      second.result.current.dismiss();
    });

    expect(installAppUpdate).toHaveBeenCalledTimes(1);
    expect(checkAppUpdate).toHaveBeenCalledTimes(1);
    expect(second.result.current.isInstalling).toBe(true);
    expect(second.result.current.progress).toEqual({ downloaded: 50, total: 100 });

    await act(async () => {
      channel.onmessage?.({ event: 'Finished' });
      channel.onmessage?.({ event: 'Progress', data: { chunkLength: 20 } });
      channel.onmessage?.({ event: 'Started', data: { contentLength: 300 } });
      installing.resolve();
      await completion;
    });
    expect(second.result.current.progress).toEqual({ downloaded: 100, total: 100 });
    expect(second.result.current.isInstalling).toBe(true);
    await act(async () => {
      second.result.current.dismiss();
      await second.result.current.downloadAndInstall();
      await second.result.current.checkForUpdate();
    });
    expect(installAppUpdate).toHaveBeenCalledTimes(1);
    expect(checkAppUpdate).toHaveBeenCalledTimes(1);
  });

  it('releases a failed install for retry and ignores progress from its old channel', async () => {
    const failedInstall = deferred<void>();
    const retryInstall = deferred<void>();
    checkAppUpdate.mockResolvedValue(AVAILABLE_UPDATE);
    installAppUpdate
      .mockReturnValueOnce(failedInstall.promise)
      .mockReturnValueOnce(retryInstall.promise);
    const first = renderHook(() => useAppUpdater());
    await act(async () => {
      await first.result.current.checkForUpdate();
    });

    let firstCompletion!: Promise<void>;
    act(() => {
      firstCompletion = first.result.current.downloadAndInstall();
    });
    const oldChannel = installAppUpdate.mock.calls[0][0];
    first.unmount();
    const { result } = renderHook(() => useAppUpdater());
    await act(async () => {
      oldChannel.onmessage?.({ event: 'Started', data: { contentLength: 100 } });
      oldChannel.onmessage?.({ event: 'Progress', data: { chunkLength: 20 } });
      failedInstall.reject(new Error('Download failed'));
      await firstCompletion;
    });
    expect(result.current.isInstalling).toBe(false);
    expect(result.current.error).toContain('Download failed');

    let retryCompletion!: Promise<void>;
    act(() => {
      retryCompletion = result.current.downloadAndInstall();
    });
    expect(installAppUpdate).toHaveBeenCalledTimes(2);
    const newChannel = installAppUpdate.mock.calls[1][0];
    act(() => {
      newChannel.onmessage?.({ event: 'Started', data: { contentLength: 200 } });
      newChannel.onmessage?.({ event: 'Progress', data: { chunkLength: 40 } });
      oldChannel.onmessage?.({ event: 'Started', data: { contentLength: 100 } });
      oldChannel.onmessage?.({ event: 'Progress', data: { chunkLength: 80 } });
      oldChannel.onmessage?.({ event: 'Finished' });
    });
    expect(result.current.isInstalling).toBe(true);
    expect(result.current.error).toBeNull();
    expect(result.current.progress).toEqual({ downloaded: 40, total: 200 });

    await act(async () => {
      retryInstall.reject(new Error('Retry failed'));
      await retryCompletion;
      oldChannel.onmessage?.({ event: 'Progress', data: { chunkLength: 100 } });
      newChannel.onmessage?.({ event: 'Progress', data: { chunkLength: 100 } });
    });
    expect(result.current.isInstalling).toBe(false);
    expect(result.current.progress).toEqual({ downloaded: 40, total: 200 });

    await act(async () => {
      await result.current.checkForUpdate();
    });
    expect(checkAppUpdate).toHaveBeenCalledTimes(2);
    expect(result.current.progress).toBeNull();
    expect(result.current.error).toBeNull();
  });

  it('does not install without an available update', async () => {
    const { result } = renderHook(() => useAppUpdater());

    await act(async () => {
      await result.current.downloadAndInstall();
    });

    expect(installAppUpdate).not.toHaveBeenCalled();
    expect(result.current.isInstalling).toBe(false);
  });
});
