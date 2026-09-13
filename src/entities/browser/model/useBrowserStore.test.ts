import { beforeEach, describe, it, expect } from 'vitest';
import { useBrowserStore } from './useBrowserStore';

describe('useBrowserStore', () => {
  beforeEach(() => {
    useBrowserStore.setState({
      gameId: null,
      tabs: [],
      activeTabId: null,
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
});
