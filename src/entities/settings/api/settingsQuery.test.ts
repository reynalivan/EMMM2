import { afterEach, describe, expect, it } from 'vitest';
import { QueryClient, isCancelledError } from '@tanstack/react-query';
import type { AppSettings } from '../model/settings';
import { publishSettingsSnapshot, settingsKeys } from './settingsQuery';

const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
const baseSettings: AppSettings = {
  revision: 1,
  theme: 'onyx',
  language: 'en',
  games: [],
  active_game_id: null,
  safety: { keywords: [] },
  ai: { enabled: false, base_url: null, has_api_key: false },
  auto_close_launcher: false,
};

afterEach(() => client.clear());

describe('authoritative settings cache publication', () => {
  it('retains a newer committed snapshot when an older response arrives later', () => {
    const current = { ...baseSettings, revision: 8, theme: 'light' };
    client.setQueryData(settingsKeys.all, current);
    publishSettingsSnapshot(client, { ...baseSettings, revision: 7 });
    expect(client.getQueryData(settingsKeys.all)).toEqual(current);
  });

  it('prevents an in-flight pre-activation read from replacing a committed snapshot', async () => {
    let resolveRead!: (settings: AppSettings) => void;
    const read = new Promise<AppSettings>((resolve) => {
      resolveRead = resolve;
    });
    const initialFetch = client
      .fetchQuery({ queryKey: settingsKeys.all, queryFn: () => read })
      .catch((error: unknown) => {
        expect(isCancelledError(error)).toBe(true);
        return undefined;
      });
    const committed = { ...baseSettings, revision: 2, active_game_id: 'game-a' };
    publishSettingsSnapshot(client, committed);
    resolveRead(baseSettings);
    await initialFetch;
    expect(client.getQueryData(settingsKeys.all)).toEqual(committed);
  });
});
