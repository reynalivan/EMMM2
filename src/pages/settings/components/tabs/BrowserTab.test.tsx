import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useToastStore } from '@/shared/ui/toast';
import BrowserTab from './BrowserTab';

vi.unmock('@tanstack/react-query');

const mocks = vi.hoisted(() => ({
  gameId: 'game-a' as string | null,
  browserGetHomepage: vi.fn(),
  browserSetHomepage: vi.fn(),
  browserGetRetentionDays: vi.fn(),
  browserSetRetentionDays: vi.fn(),
  browserClearOldDownloads: vi.fn(),
}));

vi.mock('@/shared/api/tauri/bindings', () => ({ commands: mocks }));
vi.mock('@/app/store', () => ({
  useAppStore: (selector: (state: { activeGameId: string | null }) => unknown) =>
    selector({ activeGameId: mocks.gameId }),
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

let client: QueryClient;

function renderBrowser() {
  return render(
    <QueryClientProvider client={client}>
      <BrowserTab />
    </QueryClientProvider>,
  );
}

function homepageInput() {
  return screen.getByRole('textbox', { name: /Homepage/i });
}

beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  useToastStore.setState({ toasts: [] });
  mocks.gameId = 'game-a';
  client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  mocks.browserGetHomepage.mockImplementation(async (gameId: string) =>
    gameId === 'game-b' ? 'https://fixture.invalid/b' : 'https://fixture.invalid/a',
  );
  mocks.browserSetHomepage.mockResolvedValue(undefined);
  mocks.browserGetRetentionDays.mockResolvedValue(30);
});

afterEach(() => {
  cleanup();
  client.clear();
  localStorage.clear();
});

describe('BrowserTab homepage ownership', () => {
  it('blocks previous-game blur and reset while the next homepage is loading', async () => {
    const gameB = deferred<string>();
    mocks.browserGetHomepage.mockImplementation((gameId: string) =>
      gameId === 'game-b' ? gameB.promise : Promise.resolve('https://fixture.invalid/a'),
    );
    const view = renderBrowser();
    await waitFor(() => expect(homepageInput()).toHaveValue('https://fixture.invalid/a'));
    fireEvent.change(homepageInput(), { target: { value: 'https://fixture.invalid/a-draft' } });

    mocks.gameId = 'game-b';
    view.rerender(
      <QueryClientProvider client={client}>
        <BrowserTab />
      </QueryClientProvider>,
    );
    await waitFor(() => expect(mocks.browserGetHomepage).toHaveBeenCalledWith('game-b'));
    fireEvent.blur(homepageInput());
    fireEvent.click(screen.getByRole('button', { name: /Reset/i }));
    await act(async () => {});

    expect(mocks.browserSetHomepage).not.toHaveBeenCalled();
    expect(homepageInput()).toBeDisabled();
    expect(homepageInput()).not.toHaveValue('https://fixture.invalid/a-draft');
    expect(screen.getByRole('button', { name: /Reset/i })).toBeDisabled();

    await act(async () => gameB.resolve('https://fixture.invalid/b'));
    await waitFor(() => expect(homepageInput()).toHaveValue('https://fixture.invalid/b'));
    expect(homepageInput()).toBeEnabled();
  });

  it('keeps a late game A save out of the game B cache and newer draft', async () => {
    const saveA = deferred<void>();
    mocks.browserSetHomepage.mockReturnValue(saveA.promise);
    const view = renderBrowser();
    await waitFor(() => expect(homepageInput()).toHaveValue('https://fixture.invalid/a'));
    fireEvent.change(homepageInput(), { target: { value: 'https://fixture.invalid/a-saved' } });
    fireEvent.blur(homepageInput());
    await waitFor(() =>
      expect(mocks.browserSetHomepage).toHaveBeenCalledWith(
        'game-a',
        'https://fixture.invalid/a-saved',
      ),
    );

    mocks.gameId = 'game-b';
    view.rerender(
      <QueryClientProvider client={client}>
        <BrowserTab />
      </QueryClientProvider>,
    );
    await waitFor(() => expect(homepageInput()).toHaveValue('https://fixture.invalid/b'));
    fireEvent.change(homepageInput(), { target: { value: 'https://fixture.invalid/b-draft' } });
    await act(async () => saveA.resolve(undefined));
    await waitFor(() =>
      expect(useToastStore.getState().toasts.some((toast) => toast.type === 'success')).toBe(true),
    );

    expect(client.getQueryData(['browser_homepage', 'game-a'])).toBe(
      'https://fixture.invalid/a-saved',
    );
    expect(client.getQueryData(['browser_homepage', 'game-b'])).toBe('https://fixture.invalid/b');
    expect(homepageInput()).toHaveValue('https://fixture.invalid/b-draft');
    fireEvent.blur(homepageInput());
    await waitFor(() =>
      expect(mocks.browserSetHomepage).toHaveBeenLastCalledWith(
        'game-b',
        'https://fixture.invalid/b-draft',
      ),
    );
  });

  it('keeps a newer same-game draft when a saved homepage completes', async () => {
    const save = deferred<void>();
    mocks.browserSetHomepage.mockReturnValue(save.promise);
    mocks.browserGetHomepage.mockResolvedValue('https://fixture.invalid/saved');
    renderBrowser();
    await waitFor(() => expect(homepageInput()).toHaveValue('https://fixture.invalid/saved'));
    fireEvent.change(homepageInput(), { target: { value: 'https://fixture.invalid/first' } });
    fireEvent.blur(homepageInput());
    await waitFor(() => expect(mocks.browserSetHomepage).toHaveBeenCalledOnce());
    fireEvent.change(homepageInput(), { target: { value: 'https://fixture.invalid/second' } });
    await act(async () => save.resolve(undefined));
    await waitFor(() =>
      expect(useToastStore.getState().toasts.some((toast) => toast.type === 'success')).toBe(true),
    );
    expect(homepageInput()).toHaveValue('https://fixture.invalid/second');
  });

  it('disables homepage changes when no game is selected or its read fails', async () => {
    mocks.gameId = null;
    const view = renderBrowser();
    fireEvent.click(screen.getByRole('button', { name: /Reset/i }));
    await act(async () => {});
    expect(mocks.browserSetHomepage).not.toHaveBeenCalled();
    expect(homepageInput()).toBeDisabled();

    mocks.gameId = 'game-b';
    mocks.browserGetHomepage.mockRejectedValue(new Error('Homepage unavailable'));
    view.rerender(
      <QueryClientProvider client={client}>
        <BrowserTab />
      </QueryClientProvider>,
    );
    await waitFor(() =>
      expect(client.getQueryState(['browser_homepage', 'game-b'])?.status).toBe('error'),
    );
    expect(homepageInput()).toBeDisabled();
    expect(screen.getByRole('button', { name: /Reset/i })).toBeDisabled();
  });

  it('keeps the failed homepage draft and actionable native error for retry', async () => {
    mocks.browserSetHomepage.mockRejectedValueOnce({
      type: 'Validation',
      payload: 'Choose an HTTPS URL',
    });
    renderBrowser();
    await waitFor(() => expect(homepageInput()).toHaveValue('https://fixture.invalid/a'));
    fireEvent.change(homepageInput(), { target: { value: 'https://fixture.invalid/draft' } });
    fireEvent.blur(homepageInput());
    await waitFor(() =>
      expect(
        useToastStore
          .getState()
          .toasts.some((toast) => toast.message.includes('Choose an HTTPS URL')),
      ).toBe(true),
    );
    expect(homepageInput()).toHaveValue('https://fixture.invalid/draft');
    fireEvent.blur(homepageInput());
    await waitFor(() => expect(mocks.browserSetHomepage).toHaveBeenCalledTimes(2));
  });
});

describe('BrowserTab retention migration', () => {
  it('retains legacy storage until the native migration succeeds', async () => {
    const retention = deferred<number>();
    localStorage.setItem('emmm-browser-store', JSON.stringify({ state: { retentionDays: 14 } }));
    mocks.browserGetRetentionDays.mockReturnValue(retention.promise);
    renderBrowser();
    await waitFor(() => expect(mocks.browserGetRetentionDays).toHaveBeenCalledWith(14));
    expect(localStorage.getItem('emmm-browser-store')).not.toBeNull();
    await act(async () => retention.resolve(14));
    await waitFor(() => expect(screen.getByRole('spinbutton')).toHaveValue(14));
    expect(localStorage.getItem('emmm-browser-store')).toBeNull();
  });
});
