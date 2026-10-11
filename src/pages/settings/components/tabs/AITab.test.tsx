import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { AppSettings } from '@/entities/settings';
import { useToastStore } from '@/shared/ui/toast';
import AITab from './AITab';

vi.unmock('@tanstack/react-query');

const mocks = vi.hoisted(() => ({
  getSettings: vi.fn(),
  saveSettings: vi.fn(),
  setAiApiKey: vi.fn(),
  deleteAiApiKey: vi.fn(),
  testAiConnection: vi.fn(),
}));

vi.mock('@/shared/api/tauri/bindings', () => ({ commands: mocks }));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  Channel: class {
    onmessage: ((message: unknown) => void) | undefined;
  },
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

const baseSettings: AppSettings = {
  revision: 4,
  theme: 'onyx',
  language: 'en',
  games: [],
  active_game_id: null,
  safety: { keywords: [] },
  ai: { enabled: false, base_url: 'https://fixture.invalid/original', has_api_key: false },
  auto_close_launcher: false,
};

let client: QueryClient;

function renderAI() {
  return render(
    <QueryClientProvider client={client}>
      <AITab />
    </QueryClientProvider>,
  );
}

function endpointInput() {
  return screen.getByRole('textbox', { name: /Endpoint/i });
}

function keyInput() {
  return screen.getByLabelText(/API key/i);
}

beforeEach(() => {
  vi.resetAllMocks();
  useToastStore.setState({ toasts: [] });
  client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  client.setQueryData(['settings'], baseSettings);
  mocks.getSettings.mockResolvedValue(baseSettings);
  mocks.saveSettings.mockImplementation(async (settings: AppSettings) => {
    const saved = { ...settings, revision: (settings.revision ?? 0) + 1 };
    mocks.getSettings.mockResolvedValue(saved);
    return { settings: saved, sync_warning: null };
  });
});

afterEach(() => {
  cleanup();
  client.clear();
});

describe('AITab drafts', () => {
  it('preserves an unsaved endpoint when enablement changes, then explicitly saves it', async () => {
    renderAI();
    fireEvent.change(endpointInput(), { target: { value: 'https://fixture.invalid/draft' } });
    fireEvent.click(screen.getByRole('checkbox'));
    await waitFor(() =>
      expect(client.getQueryData<AppSettings>(['settings'])?.ai.enabled).toBe(true),
    );
    expect(endpointInput()).toHaveValue('https://fixture.invalid/draft');
    expect(mocks.saveSettings.mock.calls[0]?.[0].ai.base_url).toBe(baseSettings.ai.base_url);

    fireEvent.click(screen.getByRole('button', { name: /^Save/i }));
    await waitFor(() =>
      expect(client.getQueryData<AppSettings>(['settings'])?.ai.base_url).toBe(
        'https://fixture.invalid/draft',
      ),
    );
  });

  it('retains newer endpoint and key drafts after a previous credential save completes', async () => {
    const keySave = deferred<AppSettings>();
    mocks.setAiApiKey.mockReturnValue(keySave.promise);
    renderAI();
    fireEvent.change(endpointInput(), { target: { value: 'https://fixture.invalid/first' } });
    fireEvent.change(keyInput(), { target: { value: 'fixture-first-key' } });
    fireEvent.click(screen.getByRole('button', { name: /^Save/i }));
    await waitFor(() => expect(mocks.setAiApiKey).toHaveBeenCalledWith('fixture-first-key'));
    fireEvent.change(endpointInput(), { target: { value: 'https://fixture.invalid/second' } });
    fireEvent.change(keyInput(), { target: { value: 'fixture-second-key' } });

    const saved = {
      ...baseSettings,
      revision: 6,
      ai: { ...baseSettings.ai, base_url: 'https://fixture.invalid/first', has_api_key: true },
    };
    await act(async () => keySave.resolve(saved));
    await waitFor(() =>
      expect(client.getQueryData<AppSettings>(['settings'])?.ai.has_api_key).toBe(true),
    );
    expect(endpointInput()).toHaveValue('https://fixture.invalid/second');
    expect(keyInput()).toHaveValue('fixture-second-key');

    mocks.setAiApiKey.mockResolvedValue({
      ...saved,
      revision: 8,
      ai: { ...saved.ai, base_url: 'https://fixture.invalid/second' },
    });
    fireEvent.click(screen.getByRole('button', { name: /^Save/i }));
    await waitFor(() => expect(mocks.setAiApiKey).toHaveBeenLastCalledWith('fixture-second-key'));
    await waitFor(() => expect(keyInput()).toHaveValue(''));
    expect(endpointInput()).toHaveValue('https://fixture.invalid/second');
  });

  it('preserves edits made during config save while submitting the clicked Save snapshot', async () => {
    const configSave = deferred<{ settings: AppSettings; sync_warning: null }>();
    mocks.saveSettings.mockReturnValue(configSave.promise);
    mocks.setAiApiKey.mockResolvedValue({
      ...baseSettings,
      revision: 6,
      ai: { ...baseSettings.ai, base_url: 'https://fixture.invalid/first', has_api_key: true },
    });
    renderAI();
    fireEvent.change(endpointInput(), { target: { value: 'https://fixture.invalid/first' } });
    fireEvent.change(keyInput(), { target: { value: 'fixture-first-key' } });
    fireEvent.click(screen.getByRole('button', { name: /^Save/i }));
    await waitFor(() => expect(mocks.saveSettings).toHaveBeenCalledOnce());
    fireEvent.change(endpointInput(), { target: { value: 'https://fixture.invalid/second' } });
    fireEvent.change(keyInput(), { target: { value: 'fixture-second-key' } });
    const saved = {
      ...baseSettings,
      revision: 5,
      ai: { ...baseSettings.ai, base_url: 'https://fixture.invalid/first' },
    };
    mocks.getSettings.mockResolvedValue(saved);
    await act(async () => configSave.resolve({ settings: saved, sync_warning: null }));
    await waitFor(() => expect(mocks.setAiApiKey).toHaveBeenCalledWith('fixture-first-key'));
    await waitFor(() =>
      expect(client.getQueryData<AppSettings>(['settings'])?.ai.has_api_key).toBe(true),
    );
    expect(keyInput()).toHaveValue('fixture-second-key');
    expect(endpointInput()).toHaveValue('https://fixture.invalid/second');
  });

  it('preserves a newly typed key while credential removal completes', async () => {
    const deletion = deferred<AppSettings>();
    client.setQueryData(['settings'], {
      ...baseSettings,
      ai: { ...baseSettings.ai, has_api_key: true },
    });
    mocks.deleteAiApiKey.mockReturnValue(deletion.promise);
    renderAI();
    fireEvent.click(screen.getByRole('button', { name: /Remove key/i }));
    await waitFor(() => expect(mocks.deleteAiApiKey).toHaveBeenCalledOnce());
    fireEvent.change(keyInput(), { target: { value: 'fixture-replacement-key' } });
    await act(async () => deletion.resolve({ ...baseSettings, revision: 5 }));
    await waitFor(() =>
      expect(screen.queryByRole('button', { name: /Remove key/i })).not.toBeInTheDocument(),
    );
    expect(keyInput()).toHaveValue('fixture-replacement-key');
  });

  it('keeps credential drafts and actionable native errors on failed Save', async () => {
    mocks.setAiApiKey.mockRejectedValue({ type: 'Io', payload: 'Credential store unavailable' });
    renderAI();
    fireEvent.change(keyInput(), { target: { value: 'fixture-retry-key' } });
    fireEvent.click(screen.getByRole('button', { name: /^Save/i }));
    await waitFor(() =>
      expect(
        useToastStore
          .getState()
          .toasts.some((toast) => toast.message.includes('Credential store unavailable')),
      ).toBe(true),
    );
    expect(keyInput()).toHaveValue('fixture-retry-key');
    expect(screen.getByRole('button', { name: /^Save/i })).toBeEnabled();
    mocks.setAiApiKey.mockResolvedValue({
      ...baseSettings,
      revision: 6,
      ai: { ...baseSettings.ai, has_api_key: true },
    });
    fireEvent.click(screen.getByRole('button', { name: /^Save/i }));
    await waitFor(() => expect(mocks.setAiApiKey).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(keyInput()).toHaveValue(''));
  });
});
