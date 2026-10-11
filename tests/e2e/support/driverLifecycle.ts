import {
  spawnSync,
  type ChildProcess,
  type SpawnSyncOptionsWithStringEncoding,
  type SpawnSyncReturns,
} from 'node:child_process';
import { createServer } from 'node:net';
import { setTimeout as delay } from 'node:timers/promises';

type OwnedDriver = Pick<ChildProcess, 'pid' | 'exitCode' | 'signalCode' | 'kill'>;
export type TerminateDriver = (
  command: string,
  args: readonly string[],
  options: SpawnSyncOptionsWithStringEncoding,
) => SpawnSyncReturns<string>;

type CopyFile = (source: string, target: string) => void;
type Wait = (milliseconds: number) => Promise<void>;

/** WDIO logs some hook rejections and continues; startup guards must end that process. */
export function abortNativeRun(error: unknown, teardown: () => void = () => {}): never {
  console.error('Native E2E run aborted:', error);
  try {
    teardown();
  } catch (cleanupError) {
    console.error('Owned native startup cleanup failed; evidence retained:', cleanupError);
  } finally {
    process.exit(1);
  }
}

// Windows can retain an executable lock briefly after the previous isolated
// WebView2 fixture exits. Wait for that bounded teardown window before failing.
const E2E_BINARY_COPY_ATTEMPTS = 20;
const E2E_BINARY_COPY_RETRY_DELAY_MS = 250;

function isBusyCopyError(error: unknown): boolean {
  return typeof error === 'object' && error !== null && 'code' in error && error.code === 'EBUSY';
}

/** Copies the freshly built E2E binary, retrying only a transient Windows file lock. */
export async function copyE2EBinarySnapshot(
  source: string,
  target: string,
  copy: CopyFile,
  wait: Wait = delay,
): Promise<void> {
  for (let attempt = 0; attempt < E2E_BINARY_COPY_ATTEMPTS; attempt += 1) {
    try {
      copy(source, target);
      return;
    } catch (error: unknown) {
      const finalAttempt = attempt === E2E_BINARY_COPY_ATTEMPTS - 1;
      if (!isBusyCopyError(error) || finalAttempt) throw error;
      await wait(E2E_BINARY_COPY_RETRY_DELAY_MS);
    }
  }
}

export function assertCurrentBuild(expectedDigest: string | undefined, actualDigest: string): void {
  if (!expectedDigest || expectedDigest !== actualDigest) {
    throw new Error('Refusing E2E session without the successful binary build from this run');
  }
}

export async function assertDriverPortsAvailable(ports: readonly number[]): Promise<void> {
  for (const port of ports) {
    await new Promise<void>((resolve, reject) => {
      const probe = createServer();
      probe.once('error', (error) => {
        reject(
          new Error(
            `E2E driver port ${port} is unavailable; stop its owner explicitly: ${error.message}`,
          ),
        );
      });
      probe.listen(port, '127.0.0.1', () =>
        probe.close((error) => (error ? reject(error) : resolve())),
      );
    });
  }
}

export function stopOwnedDriver(
  driver: OwnedDriver | undefined,
  terminate: TerminateDriver = spawnSync,
): void {
  if (!driver?.pid || driver.exitCode !== null || driver.signalCode !== null) return;
  if (process.platform !== 'win32') {
    driver.kill('SIGTERM');
    return;
  }
  const stopped = terminate('taskkill', ['/PID', String(driver.pid), '/T', '/F'], {
    windowsHide: true,
    encoding: 'utf8',
  });
  if (stopped.error) throw stopped.error;
  if (stopped.status !== 0) {
    throw new Error(`Could not stop owned E2E driver PID ${driver.pid}: ${stopped.stderr.trim()}`);
  }
}

export async function waitForOwnedDriver(driver: ChildProcess, url: string): Promise<void> {
  let spawnError: Error | undefined;
  const onError = (error: Error) => {
    spawnError = error;
  };
  driver.on('error', onError);
  const assertRunning = () => {
    if (spawnError) throw spawnError;
    if (driver.exitCode !== null || driver.signalCode !== null) {
      throw new Error(
        `E2E driver exited before readiness (exit=${driver.exitCode}, signal=${driver.signalCode})`,
      );
    }
  };
  const deadline = performance.now() + 30_000;
  let lastFailure = 'Driver did not report ready';
  try {
    while (performance.now() < deadline) {
      assertRunning();
      let ready = false;
      try {
        const response = await fetch(url, {
          signal: AbortSignal.timeout(1_000),
        });
        const status: unknown = await response.json();
        if (response.ok && typeof status === 'object' && status !== null && 'value' in status) {
          const value = status.value;
          if (
            typeof value === 'object' &&
            value !== null &&
            'ready' in value &&
            value.ready === true
          )
            ready = true;
        }
        lastFailure = `Driver status HTTP ${response.status} did not report ready`;
      } catch (error) {
        lastFailure = error instanceof Error ? error.message : String(error);
      }
      assertRunning();
      if (ready) return;
      await delay(100);
    }
    throw new Error(`E2E driver readiness timed out: ${lastFailure}`);
  } finally {
    driver.off('error', onError);
  }
}
