import { create } from 'zustand';
import type { ScanResultItem } from '../types/scanner';

interface ScanProgress {
  current: number;
  total: number;
  folderName: string;
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
  updateProgress: (current: number, folderName: string) => void;
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
    label: 'Initializing...',
  },
  scanResults: [],
  stats: {
    matched: 0,
    unmatched: 0,
  },

  setIsScanning: (isScanning) => set({ isScanning }),

  updateProgress: (current, folderName) =>
    set((state) => ({
      progress: {
        ...state.progress,
        current,
        folderName,
        label: `Scanning ${folderName}...`,
      },
    })),

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
        label: 'Ready to scan',
      },
      scanResults: [],
      stats: { matched: 0, unmatched: 0 },
    }),
}));
