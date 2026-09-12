import { beforeEach, describe, expect, it } from 'vitest';
import { useDedupScanStore } from './useDedupScanStore';
import { IDLE_DEDUP_SCAN_PROGRESS } from '../utils/dedupProgress';

describe('useDedupScanStore', () => {
  beforeEach(() => {
    useDedupScanStore.setState({
      gameId: null,
      progress: IDLE_DEDUP_SCAN_PROGRESS,
    });
  });

  it('retains scan progress after a page subscriber unmounts', () => {
    const store = useDedupScanStore.getState();
    store.startScan('game-1');
    store.applyEvent('game-1', {
      event: 'progress',
      data: {
        scanId: 'scan-1',
        processedFolders: 5,
        totalFolders: 10,
        currentFolder: 'Mods/Furina',
        percent: 50,
      },
    });

    expect(useDedupScanStore.getState()).toMatchObject({
      gameId: 'game-1',
      progress: {
        isScanning: true,
        scannedFolders: 5,
        totalFolders: 10,
        currentFolder: 'Mods/Furina',
      },
    });
  });

  it('does not stop a scan that belongs to another game', () => {
    const store = useDedupScanStore.getState();
    store.startScan('game-1');
    store.stopScan('game-2');

    expect(useDedupScanStore.getState().progress.isScanning).toBe(true);
  });

  it('ignores events from a scan belonging to another game', () => {
    const store = useDedupScanStore.getState();
    store.startScan('game-1');

    store.applyEvent('game-2', {
      event: 'finished',
      data: { scanId: 'scan-2', totalGroups: 0, totalMembers: 0 },
    });

    expect(useDedupScanStore.getState().progress.isScanning).toBe(true);
  });
});
