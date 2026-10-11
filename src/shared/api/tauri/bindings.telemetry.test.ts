import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { AppSettings } from './bindings.gen';
import { commands } from './bindings';

const cache = vi.hoisted(() => ({ settings: undefined as AppSettings | undefined }));
vi.mock('@/shared/lib/queryClient', () => ({
  queryClient: { getQueryData: () => cache.settings },
}));
vi.mock('@/demo/commands', () => ({
  resolveDemoCommand: () => ({ handled: false }),
}));

const snapshot: AppSettings = {
  revision: 10,
  theme: 'onyx',
  language: 'en',
  games: [],
  active_game_id: null,
  safety: { keywords: [] },
  ai: { enabled: false, has_api_key: false, base_url: null },
  auto_close_launcher: false,
  diagnostics: { telemetry_enabled: true },
};
const failure = { type: 'Validation', payload: 'fixture failure' };

beforeEach(() => {
  cache.settings = snapshot;
  vi.mocked(invoke).mockReset();
});

describe('IPC diagnostic admission ownership', () => {
  it('reports the original settings revision after opt-out and opt-in while a request waits', async () => {
    let fail = (_reason: unknown) => {};
    vi.mocked(invoke).mockImplementation((command) => {
      if (command === 'reconcile_disk_state_cmd') {
        return new Promise((_resolve, reject) => {
          fail = reject;
        });
      }
      if (command === 'record_native_error_metric') return Promise.resolve(null);
      return Promise.reject(new Error(`Unexpected IPC ${command}`));
    });
    const pending = commands.reconcileDiskStateCmd('game-a', 'ManualRepair', null, true);
    const rejected = expect(pending).rejects.toEqual(failure);
    cache.settings = { ...snapshot, revision: 11, diagnostics: { telemetry_enabled: false } };
    cache.settings = { ...snapshot, revision: 12 };
    fail(failure);
    await rejected;
    expect(invoke).toHaveBeenCalledWith('record_native_error_metric', {
      operation: 'reconcile',
      errorCode: 'validation',
      expectedRevision: 10,
    });
  });

  it('records an ordinary failure against a known enabled revision', async () => {
    vi.mocked(invoke).mockImplementation((command) =>
      command === 'record_native_error_metric' ? Promise.resolve(null) : Promise.reject(failure),
    );
    await expect(commands.getSettings()).rejects.toEqual(failure);
    expect(invoke).toHaveBeenCalledWith('record_native_error_metric', {
      operation: 'error',
      errorCode: 'validation',
      expectedRevision: 10,
    });
  });

  it.each(['disabled', 'missing'] as const)(
    'does not admit diagnostics from a %s snapshot',
    async (mode) => {
      cache.settings =
        mode === 'disabled'
          ? { ...snapshot, diagnostics: { telemetry_enabled: false } }
          : undefined;
      vi.mocked(invoke).mockRejectedValue(failure);
      await expect(commands.getSettings()).rejects.toEqual(failure);
      expect(invoke).toHaveBeenCalledTimes(1);
    },
  );
});
