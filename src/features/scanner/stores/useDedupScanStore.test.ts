import { beforeEach, describe, expect, it } from 'vitest';
import { useDedupScanStore } from './useDedupScanStore';
import { IDLE_DEDUP_SCAN_PROGRESS } from '../utils/dedupProgress';

describe('useDedupScanStore', () => {
  beforeEach(() => {
    useDedupScanStore.setState({
      gameId: null,
      generation: 0,
      scanId: null,
      progress: IDLE_DEDUP_SCAN_PROGRESS,
    });
  });

  it('retains scan progress after a page subscriber unmounts', () => {
    const store = useDedupScanStore.getState();
    const session = store.startScan('game-1')!;
    store.applyEvent(session, {
      event: 'started',
      data: { scanId: 'scan-1', gameId: 'game-1', totalFolders: 10 },
    });
    store.applyEvent(session, {
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
    const session = store.startScan('game-1')!;
    store.stopScan({ ...session, gameId: 'game-2' });

    expect(useDedupScanStore.getState().progress.isScanning).toBe(true);
  });

  it('ignores events from a scan belonging to another game', () => {
    const store = useDedupScanStore.getState();
    const session = store.startScan('game-1')!;

    store.applyEvent(
      { ...session, gameId: 'game-2' },
      {
        event: 'finished',
        data: { scanId: 'scan-2', totalGroups: 0, totalMembers: 0 },
      },
    );

    expect(useDedupScanStore.getState().progress.isScanning).toBe(true);
  });

  it('ignores events with the wrong scan id for the current game', () => {
    const store = useDedupScanStore.getState();
    const session = store.startScan('game-1')!;
    store.applyEvent(session, {
      event: 'started',
      data: { scanId: 'current-scan', gameId: 'game-1', totalFolders: 10 },
    });
    store.applyEvent(session, {
      event: 'finished',
      data: { scanId: 'old-scan', totalGroups: 0, totalMembers: 0 },
    });

    expect(useDedupScanStore.getState().progress.isScanning).toBe(true);
  });

  it('does not let a delayed started event revive a terminal scan', () => {
    const store = useDedupScanStore.getState();
    const session = store.startScan('game-1')!;
    store.applyEvent(session, {
      event: 'started',
      data: { scanId: 'scan-1', gameId: 'game-1', totalFolders: 10 },
    });
    store.applyEvent(session, {
      event: 'cancelled',
      data: { scanId: 'scan-1', processedFolders: 0, totalFolders: 10 },
    });
    store.applyEvent(session, {
      event: 'started',
      data: { scanId: 'scan-1', gameId: 'game-1', totalFolders: 10 },
    });

    expect(useDedupScanStore.getState().progress.isScanning).toBe(false);
  });

  it('rejects another start while a scan is running', () => {
    const store = useDedupScanStore.getState();
    const session = store.startScan('game-1')!;

    expect(store.startScan('game-2')).toBeNull();
    expect(useDedupScanStore.getState().gameId).toBe(session.gameId);
  });

  it('ignores callbacks from an older request while the next scan is starting', () => {
    const store = useDedupScanStore.getState();
    const oldSession = store.startScan('game-1')!;
    store.stopScan(oldSession);
    const session = store.startScan('game-1')!;

    store.applyEvent(oldSession, {
      event: 'started',
      data: { scanId: 'old-scan', gameId: 'game-1', totalFolders: 10 },
    });
    store.stopScan(oldSession);
    expect(useDedupScanStore.getState()).toMatchObject({
      generation: session.generation,
      scanId: null,
      progress: { isScanning: true, totalFolders: 0 },
    });

    store.applyEvent(session, {
      event: 'started',
      data: { scanId: 'current-scan', gameId: 'game-1', totalFolders: 4 },
    });
    expect(useDedupScanStore.getState().scanId).toBe('current-scan');
  });

  it('records a failure received before Started', () => {
    const store = useDedupScanStore.getState();
    const session = store.startScan('game-1')!;
    store.applyEvent(session, {
      event: 'failed',
      data: { scanId: 'scan-1', processedFolders: 0, totalFolders: 0, message: 'walk failed' },
    });

    expect(useDedupScanStore.getState().progress).toMatchObject({
      isScanning: false,
      error: 'walk failed',
    });
  });
});
