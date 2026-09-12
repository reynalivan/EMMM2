import { beforeEach, describe, it, expect } from 'vitest';
import { useBrowserStore } from './useBrowserStore';

describe('useBrowserStore', () => {
  beforeEach(() => {
    useBrowserStore.setState({
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

});
