import { create } from 'zustand';

interface AppState {
  /** Privacy mode: true = SFW only. Always true on startup (TRD §5.2) */
  safeMode: boolean;
  /** Currently selected game ID */
  activeGameId: string | null;
  /** Whether this is the first app launch (no config) */
  isFirstRun: boolean;
  /** Loading state for startup check */
  isLoading: boolean;

  setSafeMode: (v: boolean) => void;
  setActiveGameId: (id: string | null) => void;
  setFirstRun: (v: boolean) => void;
  setLoading: (v: boolean) => void;
}

export const useAppStore = create<AppState>()((set) => ({
  safeMode: true,
  activeGameId: null,
  isFirstRun: true,
  isLoading: true,

  setSafeMode: (v) => set({ safeMode: v }),
  setActiveGameId: (id) => set({ activeGameId: id }),
  setFirstRun: (v) => set({ isFirstRun: v }),
  setLoading: (v) => set({ isLoading: v }),
}));
