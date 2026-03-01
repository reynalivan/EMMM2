import { describe, it, expect } from 'vitest';
import { useBrowserStore } from './useBrowserStore';

describe('useBrowserStore', () => {
  it('maintains default state properly', () => {
    const state = useBrowserStore.getState();
    expect(state.isDownloadPanelOpen).toBe(false);
    expect(state.selectedDownloadIds.size).toBe(0);
  });

  it('toggles the download panel visibility', () => {
    const store = useBrowserStore.getState();
    store.toggleDownloadPanel();
    expect(useBrowserStore.getState().isDownloadPanelOpen).toBe(true);

    useBrowserStore.getState().toggleDownloadPanel();
    expect(useBrowserStore.getState().isDownloadPanelOpen).toBe(false);
  });

  it('adds and removes items from selected downloads', () => {
    useBrowserStore.getState().toggleSelectDownload('dl-1');
    expect(useBrowserStore.getState().selectedDownloadIds.has('dl-1')).toBe(true);
    expect(useBrowserStore.getState().selectedDownloadIds.size).toBe(1);

    useBrowserStore.getState().toggleSelectDownload('dl-1');
    expect(useBrowserStore.getState().selectedDownloadIds.has('dl-1')).toBe(false);
    expect(useBrowserStore.getState().selectedDownloadIds.size).toBe(0);
  });

  it('selects all given ids to the downloads', () => {
    useBrowserStore.getState().selectAll(['dl-1', 'dl-2', 'dl-3']);
    const state = useBrowserStore.getState();
    expect(state.selectedDownloadIds.size).toBe(3);
    expect(state.selectedDownloadIds.has('dl-1')).toBe(true);
    expect(state.selectedDownloadIds.has('dl-2')).toBe(true);
    expect(state.selectedDownloadIds.has('dl-3')).toBe(true);
  });

  it('clears all selected downloads', () => {
    useBrowserStore.getState().selectAll(['dl-1', 'dl-2', 'dl-3']);
    useBrowserStore.getState().clearSelection();

    expect(useBrowserStore.getState().selectedDownloadIds.size).toBe(0);
  });
});
