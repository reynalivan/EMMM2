import { ChildProcess } from 'node:child_process';
import { createServer } from 'node:net';
import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  assertDriverPortsAvailable,
  assertCurrentBuild,
  stopOwnedDriver,
  waitForOwnedDriver,
  type TerminateDriver,
} from '../../tests/e2e/support/driverLifecycle';

afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetAllMocks();
});

describe('native E2E driver ownership', () => {
  it('rejects a stale binary or failed build even when the old binary exists', () => {
    expect(() => assertCurrentBuild(undefined, 'old')).toThrow('successful binary build');
    expect(() => assertCurrentBuild('new', 'old')).toThrow('successful binary build');
    expect(() => assertCurrentBuild('new', 'new')).not.toThrow();
  });
  it('rejects an occupied port without stopping its owner', async () => {
    const other = createServer();
    await new Promise<void>((resolve) => other.listen(0, '127.0.0.1', resolve));
    const address = other.address();
    if (!address || typeof address === 'string') throw new Error('Expected TCP address');
    try {
      await expect(assertDriverPortsAvailable([address.port])).rejects.toThrow(
        'stop its owner explicitly',
      );
      expect(other.listening).toBe(true);
    } finally {
      await new Promise<void>((resolve, reject) =>
        other.close((error) => (error ? reject(error) : resolve())),
      );
    }
  });

  it('only stops the live child PID, never a process image name', () => {
    const terminate = vi.fn<TerminateDriver>().mockReturnValue({
      pid: 1,
      output: [],
      stdout: '',
      stderr: '',
      status: 0,
      signal: null,
    });
    const owned = { pid: 98765, exitCode: null, signalCode: null, kill: vi.fn() };
    stopOwnedDriver(owned, terminate);
    if (process.platform === 'win32') {
      expect(terminate).toHaveBeenCalledWith(
        'taskkill',
        ['/PID', '98765', '/T', '/F'],
        expect.objectContaining({ windowsHide: true }),
      );
    } else {
      expect(owned.kill).toHaveBeenCalledWith('SIGTERM');
    }
  });

  it('does not kill an exited child or a missing child', () => {
    const owned = { pid: 98765, exitCode: 0, signalCode: null, kill: vi.fn() };
    const terminate = vi.fn<TerminateDriver>();
    stopOwnedDriver(owned, terminate);
    stopOwnedDriver(undefined, terminate);
    expect(terminate).not.toHaveBeenCalled();
    expect(owned.kill).not.toHaveBeenCalled();
  });

  it('fails readiness when the launched driver exits', async () => {
    const owned = new ChildProcess();
    Object.defineProperty(owned, 'exitCode', { value: 1, configurable: true });
    const fetchStatus = vi.fn();
    vi.stubGlobal('fetch', fetchStatus);
    await expect(waitForOwnedDriver(owned, 'http://127.0.0.1:4444/status')).rejects.toThrow(
      'exited before readiness',
    );
    expect(fetchStatus).not.toHaveBeenCalled();
    expect(owned.listenerCount('error')).toBe(0);
  });

  it('waits for ready instead of accepting any HTTP response', async () => {
    const owned = new ChildProcess();
    const fetchStatus = vi
      .fn()
      .mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => ({ value: { ready: false } }),
      })
      .mockResolvedValueOnce({
        ok: true,
        status: 200,
        json: async () => ({ value: { ready: true } }),
      });
    vi.stubGlobal('fetch', fetchStatus);
    await waitForOwnedDriver(owned, 'http://127.0.0.1:4444/status');
    expect(fetchStatus).toHaveBeenCalledTimes(2);
    expect(owned.listenerCount('error')).toBe(0);
  });

  it('rejects a ready response if the owned driver exits while awaiting it', async () => {
    const owned = new ChildProcess();
    vi.stubGlobal(
      'fetch',
      vi.fn().mockImplementation(async () => {
        Object.defineProperty(owned, 'exitCode', { value: 1, configurable: true });
        return { ok: true, status: 200, json: async () => ({ value: { ready: true } }) };
      }),
    );
    await expect(waitForOwnedDriver(owned, 'http://127.0.0.1:4444/status')).rejects.toThrow(
      'exited before readiness',
    );
    expect(owned.listenerCount('error')).toBe(0);
  });
});
