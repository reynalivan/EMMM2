import { browser } from '@wdio/globals';

/**
 * Direct Tauri IPC bridge for E2E — invokes a Rust command bypassing the UI.
 * Use for seeding state and for flows that can't be clicked (native pickers,
 * game launch, archive passwords). Fails fast: throws with the backend error
 * string instead of swallowing it.
 */

interface TauriWindow extends Window {
  __TAURI__: {
    core: {
      invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
    };
  };
}

type InternalInvoke = (cmd: string, args?: unknown, options?: unknown) => Promise<unknown>;

export interface IpcRequest {
  cmd: string;
  args?: Record<string, unknown>;
}

export interface IpcSettledResult<T = unknown> {
  status: 'fulfilled' | 'rejected';
  value?: T;
  error?: unknown;
}

export interface IpcCaptureRecord<T = unknown> extends IpcSettledResult<T> {
  started_at_ms: number;
  settled_at_ms: number;
}

export type AsyncIpcState<T = unknown> =
  | {
      status: 'pending';
      started_at_ms: number;
      current_js_heap_bytes: number | null;
    }
  | {
      status: 'fulfilled';
      value: T;
      error?: undefined;
      started_at_ms: number;
      settled_at_ms: number;
      current_js_heap_bytes: number | null;
      settled_js_heap_bytes: number | null;
    }
  | {
      status: 'rejected';
      value?: undefined;
      error: unknown;
      started_at_ms: number;
      settled_at_ms: number;
      current_js_heap_bytes: number | null;
      settled_js_heap_bytes: number | null;
    };

interface StoredAsyncIpcState {
  status: 'pending' | 'fulfilled' | 'rejected';
  started_at_ms: number;
  settled_at_ms?: number;
  settled_js_heap_bytes?: number | null;
  value?: unknown;
  error?: unknown;
}

interface CapturingTauriWindow extends TauriWindow {
  __TAURI_INTERNALS__: { invoke: InternalInvoke };
  __EMMM_E2E_NEXT_FRAME_MS__?: number | null;
  __EMMM_E2E_ASYNC_IPC__?: Record<string, StoredAsyncIpcState>;
}

/**
 * Backend rejections are `AppError` objects, so `String(error)` would flatten
 * every distinct failure into "[object Object]" and hide what actually broke.
 */
function formatIpcError(error: unknown): string {
  if (typeof error === 'string') return error;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

export async function invokeInApp<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const serialized = (await browser.executeAsync(
    (payload: string, done: (r: string) => void) => {
      const { cmd, args } = JSON.parse(payload) as IpcRequest;
      const { invoke } = (window as unknown as TauriWindow).__TAURI__.core;
      invoke(cmd, args).then(
        (value) => done(JSON.stringify({ ok: true, value })),
        (error) =>
          done(
            JSON.stringify({ ok: false, error: error instanceof Error ? error.message : error }),
          ),
      );
    },
    JSON.stringify({ cmd, args: args ?? {} }),
  )) as string;
  const result = JSON.parse(serialized) as { ok: boolean; value?: T; error?: unknown };

  if (!result.ok) {
    throw new Error(`[IPC] ${cmd} failed: ${formatIpcError(result.error)}`);
  }
  return result.value as T;
}

/** Runs a bounded synthetic in-WebView IPC burst and accounts for every outcome. */
export async function invokeManyInApp<T = unknown>(
  requests: readonly IpcRequest[],
): Promise<IpcCaptureRecord<T>[]> {
  const serialized = (await browser.executeAsync(
    (serializedBatch: string, done: (settled: string) => void) => {
      const batch = JSON.parse(serializedBatch) as IpcRequest[];
      const { invoke } = (window as unknown as TauriWindow).__TAURI__.core;
      Promise.all(
        batch.map(async ({ cmd, args }): Promise<IpcCaptureRecord> => {
          const startedAt = performance.now();
          try {
            const value = await invoke(cmd, args);
            return {
              status: 'fulfilled',
              value,
              started_at_ms: startedAt,
              settled_at_ms: performance.now(),
            };
          } catch (error: unknown) {
            return {
              status: 'rejected',
              error: error instanceof Error ? error.message : error,
              started_at_ms: startedAt,
              settled_at_ms: performance.now(),
            };
          }
        }),
      ).then((settled) => done(JSON.stringify(settled)));
    },
    JSON.stringify(requests),
  )) as string;
  return JSON.parse(serialized) as IpcCaptureRecord<T>[];
}

/** Starts native IPC without holding the WebDriver command open. */
export async function startAsyncIpcInApp(
  key: string,
  cmd: string,
  args?: Record<string, unknown>,
): Promise<void> {
  await browser.execute(
    (payload: string) => {
      const {
        key: operationKey,
        cmd: command,
        args: commandArgs,
      } = JSON.parse(payload) as IpcRequest & { key: string };
      const target = window as unknown as CapturingTauriWindow;
      const operations = (target.__EMMM_E2E_ASYNC_IPC__ ??= {});
      if (operations[operationKey]?.status === 'pending') {
        throw new Error(`Async IPC operation ${operationKey} is already pending`);
      }

      const state: StoredAsyncIpcState = {
        status: 'pending',
        started_at_ms: performance.now(),
      };
      operations[operationKey] = state;
      void target.__TAURI_INTERNALS__.invoke(command, commandArgs).then(
        (value) => {
          const memory = (performance as Performance & { memory?: { usedJSHeapSize: number } })
            .memory;
          state.status = 'fulfilled';
          state.value = value;
          state.settled_at_ms = performance.now();
          state.settled_js_heap_bytes = memory?.usedJSHeapSize ?? null;
        },
        (error: unknown) => {
          const memory = (performance as Performance & { memory?: { usedJSHeapSize: number } })
            .memory;
          state.status = 'rejected';
          state.error = error instanceof Error ? error.message : error;
          state.settled_at_ms = performance.now();
          state.settled_js_heap_bytes = memory?.usedJSHeapSize ?? null;
        },
      );
    },
    JSON.stringify({ key, cmd, args: args ?? {} }),
  );
}

export async function readAsyncIpcInApp<T = unknown>(key: string): Promise<AsyncIpcState<T>> {
  const serialized = (await browser.execute((operationKey: string) => {
    const target = window as unknown as CapturingTauriWindow;
    const state = target.__EMMM_E2E_ASYNC_IPC__?.[operationKey];
    if (!state) {
      throw new Error(`Async IPC operation ${operationKey} does not exist`);
    }
    const memory = (performance as Performance & { memory?: { usedJSHeapSize: number } }).memory;
    const currentHeap = memory?.usedJSHeapSize ?? null;
    if (state.status === 'pending') {
      return JSON.stringify({
        status: 'pending',
        started_at_ms: state.started_at_ms,
        current_js_heap_bytes: currentHeap,
      });
    }
    return JSON.stringify({
      status: state.status,
      value: state.value,
      error: state.error,
      started_at_ms: state.started_at_ms,
      settled_at_ms: state.settled_at_ms,
      current_js_heap_bytes: currentHeap,
      settled_js_heap_bytes: state.settled_js_heap_bytes ?? null,
    });
  }, key)) as string;
  return JSON.parse(serialized) as AsyncIpcState<T>;
}

export async function clearAsyncIpcInApp(key: string): Promise<void> {
  await browser.execute((operationKey: string) => {
    const operations = (window as unknown as CapturingTauriWindow).__EMMM_E2E_ASYNC_IPC__;
    if (operations?.[operationKey]?.status === 'pending') {
      throw new Error(`Cannot clear pending async IPC operation ${operationKey}`);
    }
    if (operations) {
      delete operations[operationKey];
    }
  }, key);
}

/** Arms a next-animation-frame proxy for the next DOM click (not actual paint). */
export async function armNextFrameProxy(): Promise<void> {
  await browser.execute(() => {
    const target = window as unknown as CapturingTauriWindow;
    target.__EMMM_E2E_NEXT_FRAME_MS__ = null;
    window.addEventListener(
      'click',
      () => {
        const clickedAt = performance.now();
        requestAnimationFrame(() => {
          target.__EMMM_E2E_NEXT_FRAME_MS__ = performance.now() - clickedAt;
        });
      },
      { capture: true, once: true },
    );
  });
}

export async function waitForNextFrameProxy(timeout = 2_000): Promise<number> {
  let measured: number | null = null;
  await browser.waitUntil(
    async () => {
      measured = (await browser.execute(
        () => (window as unknown as CapturingTauriWindow).__EMMM_E2E_NEXT_FRAME_MS__ ?? null,
      )) as number | null;
      return measured !== null;
    },
    { timeout, interval: 25, timeoutMsg: 'Next-frame proxy did not produce a sample' },
  );
  if (measured === null) {
    throw new Error('Next-frame proxy completed without a sample');
  }
  return measured;
}

/**
 * Invokes a command that streams progress through a Tauri `Channel` (scanner,
 * archive extraction, dedup). Constructs the channel in-page, collects every
 * emitted event, and resolves with both the final result and the events.
 */
export async function invokeWithChannel<T = unknown>(
  cmd: string,
  args: Record<string, unknown>,
  channelKey: string,
): Promise<{ value: T; events: unknown[] }> {
  const result = (await browser.executeAsync(
    (c: string, a: Record<string, unknown>, key: string, done: (r: unknown) => void) => {
      const core = (
        window as unknown as TauriWindow & {
          __TAURI__: { core: { Channel: new () => { onmessage: (m: unknown) => void } } };
        }
      ).__TAURI__.core;
      const channel = new core.Channel();
      const events: unknown[] = [];
      channel.onmessage = (m: unknown) => events.push(m);
      const invoke = (
        core as unknown as {
          invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
        }
      ).invoke;
      invoke(c, { ...a, [key]: channel }).then(
        (value) => done({ ok: true, value, events }),
        (error) =>
          done({ ok: false, error: error instanceof Error ? error.message : error, events }),
      );
    },
    cmd,
    args,
    channelKey,
  )) as { ok: boolean; value?: T; error?: unknown; events: unknown[] };

  if (!result.ok) {
    throw new Error(`[IPC] ${cmd} (channel) failed: ${formatIpcError(result.error)}`);
  }
  return { value: result.value as T, events: result.events };
}
