import { create } from 'zustand';
import type { DupScanEvent } from '@/entities/workspace';
import {
  IDLE_DEDUP_SCAN_PROGRESS,
  reduceDedupProgress,
  type DedupScanProgress,
} from '../utils/dedupProgress';

interface ScanSession {
  gameId: string;
  generation: number;
}

interface DedupScanStore {
  gameId: string | null;
  generation: number;
  scanId: string | null;
  progress: DedupScanProgress;
  startScan: (gameId: string) => ScanSession | null;
  applyEvent: (session: ScanSession, event: DupScanEvent) => boolean;
  stopScan: (session: ScanSession) => void;
}

export const useDedupScanStore = create<DedupScanStore>((set, get) => ({
  gameId: null,
  generation: 0,
  scanId: null,
  progress: IDLE_DEDUP_SCAN_PROGRESS,

  startScan: (gameId) => {
    const current = get();
    if (current.progress.isScanning) return null;
    const session = { gameId, generation: current.generation + 1 };
    set({
      ...session,
      scanId: null,
      progress: { ...IDLE_DEDUP_SCAN_PROGRESS, isScanning: true },
    });
    return session;
  },

  applyEvent: (session, event) => {
    const current = get();
    if (!ownsSession(current, session) || !current.progress.isScanning) return false;
    if (event.event === 'started' && event.data.gameId !== session.gameId) return false;
    if (current.scanId !== null) {
      if (event.event === 'started' || current.scanId !== event.data.scanId) return false;
    } else if (event.event !== 'started' && event.event !== 'failed') {
      return false;
    }
    set({
      scanId: event.data.scanId,
      progress: reduceDedupProgress(current.progress, event),
    });
    return true;
  },

  stopScan: (session) => {
    set((current) =>
      ownsSession(current, session)
        ? { progress: { ...current.progress, isScanning: false } }
        : current,
    );
  },
}));

function ownsSession(current: DedupScanStore, session: ScanSession): boolean {
  return current.gameId === session.gameId && current.generation === session.generation;
}
