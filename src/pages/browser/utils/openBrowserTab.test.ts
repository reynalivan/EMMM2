import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useBrowserStore } from '@/entities/browser';
import { commands } from '@/shared/api/tauri/bindings';
import { openBrowserTabForSession } from './openBrowserTab';

const activeGame = { id: 'game-1' };
const closeWebview = vi.fn().mockResolvedValue(undefined);
vi.mock('@/app/store', () => ({
  useAppStore: { getState: () => ({ activeGameId: activeGame.id }) },
}));
vi.mock('@/shared/api/tauri/bindings', () => ({ commands: { browserOpenTab: vi.fn() } }));
vi.mock('@tauri-apps/api/webview', () => ({
  Webview: { getByLabel: vi.fn(async () => ({ close: closeWebview })) },
}));

describe('openBrowserTabForSession', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    activeGame.id = 'game-1';
    useBrowserStore.setState({ gameId: 'game-1', generation: 1, tabs: [], activeTabId: null });
  });

  it('closes a newly created webview instead of publishing it into another game', async () => {
    let finish: (label: string) => void = () => {
      throw new Error('Open was not called');
    };
    vi.mocked(commands.browserOpenTab).mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const pending = openBrowserTabForSession({ url: 'https://example.com', title: 'Example' });
    activeGame.id = 'game-2';
    useBrowserStore.getState().setGameContext('game-2');
    finish('browser-tab-old');

    expect(await pending).toBeNull();
    expect(useBrowserStore.getState().tabs).toEqual([]);
    expect(closeWebview).toHaveBeenCalledOnce();
  });

  it('rejects a late result after switching away and back to the same game', async () => {
    let finish: (label: string) => void = () => {
      throw new Error('Open was not called');
    };
    vi.mocked(commands.browserOpenTab).mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const pending = openBrowserTabForSession({ url: 'https://example.com', title: 'Example' });
    useBrowserStore.getState().setGameContext('game-2');
    useBrowserStore.getState().setGameContext('game-1');
    finish('browser-tab-old');
    expect(await pending).toBeNull();
    expect(useBrowserStore.getState().tabs).toEqual([]);
  });
});
