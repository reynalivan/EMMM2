import { create } from 'zustand';
import type { DupScanEvent } from '@/entities/workspace';
import {
  IDLE_DEDUP_SCAN_PROGRESS,
  reduceDedupProgress,
  type DedupScanProgress,
} from '../utils/dedupProgress';

interface DedupScanStore {
  gameId: string | null;
  progress: DedupScanProgress;
  startScan: (gameId: string) => void;
  applyEvent: (gameId: string, event: DupScanEvent) => void;
  stopScan: (gameId: string) => void;
}

export const useDedupScanStore = create<DedupScanStore>((set) => ({
  gameId: null,
  progress: IDLE_DEDUP_SCAN_PROGRESS,

  startScan: (gameId) => {
    set({
      gameId,
      progress: { ...IDLE_DEDUP_SCAN_PROGRESS, isScanning: true },
    });
  },

  applyEvent: (gameId, event) => {
    set((current) =>
      current.gameId === gameId
        ? { progress: reduceDedupProgress(current.progress, event) }
        : current,
    );
  },

  stopScan: (gameId) => {
    set((current) =>
      current.gameId === gameId
        ? { progress: { ...current.progress, isScanning: false } }
        : current,
    );
  },
}));
