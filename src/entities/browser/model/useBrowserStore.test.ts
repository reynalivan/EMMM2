import { beforeEach, describe, it, expect } from 'vitest';
import { createNewBrowserTab, useBrowserStore } from './useBrowserStore';

describe('useBrowserStore', () => {
  beforeEach(() => {
    useBrowserStore.setState({
      gameId: null,
      tabs: [],
      activeTabId: null,
      recentlyClosedTabs: [],
      isDownloadPanelOpen: false,
      isDownloadConfirmationOpen: false,
    });
  });

  it('maintains default state properly', () => {
    const state = useBrowserStore.getState();
    expect(state.isDownloadPanelOpen).toBe(false);
  });

  it('toggles the download panel visibility', () => {
    const store = useBrowserStore.getState();
    store.toggleDownloadPanel();
    expect(useBrowserStore.getState().isDownloadPanelOpen).toBe(true);

    useBrowserStore.getState().toggleDownloadPanel();
    expect(useBrowserStore.getState().isDownloadPanelOpen).toBe(false);
  });

  it('clears tabs when the active game changes', () => {
    const store = useBrowserStore.getState();
    store.setGameContext('game-1');
    store.addTab({ id: 'tab-1', url: 'https://example.test', title: 'Example' });

    useBrowserStore.getState().setGameContext('game-2');

    expect(useBrowserStore.getState()).toMatchObject({
      gameId: 'game-2',
      tabs: [],
      activeTabId: null,
    });
  });

  it('replaces a local New Tab with its native webview tab', () => {
    const store = useBrowserStore.getState();
    const newTab = createNewBrowserTab();
    store.addTab(newTab);

    store.replaceTab(newTab.id, {
      id: 'browser-tab-1',
      url: 'https://example.test',
      title: 'Example',
    });

    expect(useBrowserStore.getState()).toMatchObject({
      activeTabId: 'browser-tab-1',
      tabs: [{ id: 'browser-tab-1', url: 'https://example.test' }],
    });
  });

  it('keeps one local New Tab when the final tab closes', () => {
    const store = useBrowserStore.getState();
    store.addTab({ id: 'browser-tab-1', url: 'https://example.test', title: 'Example' });

    store.removeTab('browser-tab-1');

    const state = useBrowserStore.getState();
    expect(state.tabs).toHaveLength(1);
    expect(state.tabs[0]).toMatchObject({ isNewTab: true, url: '' });
    expect(state.activeTabId).toBe(state.tabs[0]?.id);
  });

  it('keeps a bounded, game-scoped history of closed web tabs', () => {
    const store = useBrowserStore.getState();
    const closedTab = {
      id: 'browser-tab-1',
      url: 'https://example.test',
      title: 'Example',
      isLoading: true,
    };

    store.recordClosedTab(closedTab);

    expect(useBrowserStore.getState().recentlyClosedTabs).toEqual([
      { ...closedTab, isLoading: false },
    ]);

    store.setGameContext('game-2');
    expect(useBrowserStore.getState().recentlyClosedTabs).toEqual([]);
  });
});
