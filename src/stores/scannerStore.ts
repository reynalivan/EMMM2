import { create } from 'zustand';
import type { ScanResultItem } from '../types/scanner';

interface ScanProgress {
  current: number;
  total: number;
  folderName: string;
  etaMs: number;
  label: string;
}

interface ScannerState {
  isScanning: boolean;
  progress: ScanProgress;
  scanResults: ScanResultItem[];
  stats: {
    matched: number;
    unmatched: number;
  };

  // Actions
  setIsScanning: (isScanning: boolean) => void;
  updateProgress: (current: number, folderName: string, etaMs: number) => void;
  setTotalFolders: (total: number) => void;
  addScanResult: (result: ScanResultItem) => void;
  setScanResults: (results: ScanResultItem[]) => void;
  setStats: (matched: number, unmatched: number) => void;
  resetScanner: () => void;
}

export const useScannerStore = create<ScannerState>()((set) => ({
  isScanning: false,
  progress: {
    current: 0,
    total: 0,
    folderName: '',
    etaMs: 0,
    label: 'Initializing...',
  },
  scanResults: [],
  stats: {
    matched: 0,
    unmatched: 0,
  },

  setIsScanning: (isScanning) => set({ isScanning }),

  updateProgress: (current, folderName, etaMs) =>
    set((state) => {
      let etaLabel = '';
      if (etaMs > 0) {
        const secs = Math.ceil(etaMs / 1000);
        etaLabel = secs >= 60 ? ` — ~${Math.ceil(secs / 60)}m remaining` : ` — ~${secs}s remaining`;
      }
      return {
        progress: {
          ...state.progress,
          current,
          folderName,
          etaMs,
          label: `Scanning ${folderName}...${etaLabel}`,
        },
      };
    }),

  setTotalFolders: (total) =>
    set((state) => ({
      progress: {
        ...state.progress,
        total,
        label: `Found ${total} folders to scan`,
      },
    })),

  addScanResult: (result) =>
    set((state) => ({
      scanResults: [...state.scanResults, result],
    })),

  setScanResults: (results) => set({ scanResults: results }),

  setStats: (matched, unmatched) => set({ stats: { matched, unmatched } }),

  resetScanner: () =>
    set({
      isScanning: false,
      progress: {
        current: 0,
        total: 0,
        folderName: '',
        etaMs: 0,
        label: 'Ready to scan',
      },
      scanResults: [],
      stats: { matched: 0, unmatched: 0 },
    }),
}));
