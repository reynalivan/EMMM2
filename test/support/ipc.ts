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
  const result = (await browser.executeAsync(
    (c: string, a: Record<string, unknown>, done: (r: unknown) => void) => {
      const { invoke } = (window as unknown as TauriWindow).__TAURI__.core;
      invoke(c, a).then(
        (value) => done({ ok: true, value }),
        (error) => done({ ok: false, error: error instanceof Error ? error.message : error }),
      );
    },
    cmd,
    args ?? {},
  )) as { ok: boolean; value?: T; error?: unknown };

  if (!result.ok) {
    throw new Error(`[IPC] ${cmd} failed: ${formatIpcError(result.error)}`);
  }
  return result.value as T;
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
