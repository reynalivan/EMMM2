import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ReactNode } from 'react';
import { useSettings } from './useSettings';
import type { AppSettings } from '../model/settings';
import { useToastStore } from '@/shared/ui/toast';

vi.unmock('@tanstack/react-query');

const settings: AppSettings = {
  revision: 4,
  theme: 'onyx',
  language: 'en',
  games: [],
  active_game_id: null,
  safety: { keywords: [] },
  ai: { enabled: false, base_url: null, has_api_key: false },
  auto_close_launcher: false,
};
const revisionMessage =
  'Settings changed since this screen was loaded. Refresh and retry your edit.';

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  useToastStore.setState({ toasts: [] });
  vi.mocked(invoke).mockImplementation(async (command) => {
    if (command === 'get_settings') return settings;
    if (command === 'save_settings') throw { type: 'Validation', payload: revisionMessage };
    throw new Error(`Unexpected command: ${command}`);
  });
});

describe('settings typed error feedback', () => {
  it('shows the actionable native error payload rather than object stringification', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );
    const hook = renderHook(() => useSettings(), { wrapper });
    await waitFor(() => expect(hook.result.current.settings).toEqual(settings));
    await act(async () => {
      await expect(hook.result.current.saveSettingsAsync(settings)).rejects.toMatchObject({
        type: 'Validation',
        payload: revisionMessage,
      });
    });
    const toasts = useToastStore.getState().toasts;
    const message = toasts[toasts.length - 1]?.message;
    expect(message).toContain(revisionMessage);
    expect(message).not.toContain('[object Object]');
    hook.unmount();
    client.clear();
  });
});
