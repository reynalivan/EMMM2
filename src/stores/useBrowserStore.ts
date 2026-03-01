import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface BrowserTab {
  id: string; // The webview label
  url: string;
  title: string;
}

interface BrowserStore {
  /** The open browser tabs backing the MultiWebview */
  tabs: BrowserTab[];
  activeTabId: string | null;

  addTab: (tab: BrowserTab) => void;
  removeTab: (id: string) => void;
  setActiveTab: (id: string) => void;
  updateTab: (id: string, updates: Partial<BrowserTab>) => void;

  /** Whether the Download Manager slide-in panel is open. */
  isDownloadPanelOpen: boolean;
  /** IDs of finished downloads selected for bulk import. */
  selectedDownloadIds: Set<string>;

  toggleDownloadPanel: () => void;
  openDownloadPanel: () => void;
  closeDownloadPanel: () => void;

  /** Toggle one download ID in the selection set. */
  toggleSelectDownload: (id: string) => void;
  /** Replace selection with an explicit set of IDs. */
  selectAll: (ids: string[]) => void;
  clearSelection: () => void;

  // --- Settings ---
  autoImport: boolean;
  skipGamePicker: boolean;
  allowedExtensions: string[];
  retentionDays: number;
  downloadsRoot: string;

  setAutoImport: (val: boolean) => void;
  setSkipGamePicker: (val: boolean) => void;
  setAllowedExtensions: (exts: string[]) => void;
  setRetentionDays: (days: number) => void;
  setDownloadsRoot: (path: string) => void;
}

export const useBrowserStore = create<BrowserStore>()(
  persist(
    (set) => ({
      tabs: [],
      activeTabId: null,

      addTab: (tab) =>
        set((s) => ({
          tabs: [...s.tabs, tab],
          activeTabId: tab.id,
        })),

      removeTab: (id) =>
        set((s) => {
          const nextTabs = s.tabs.filter((t) => t.id !== id);
          let nextActive = s.activeTabId;
          // If we closed the active tab, pick the previous one
          if (nextActive === id && nextTabs.length > 0) {
            nextActive = nextTabs[nextTabs.length - 1].id;
          } else if (nextTabs.length === 0) {
            nextActive = null;
          }
          return { tabs: nextTabs, activeTabId: nextActive };
        }),

      setActiveTab: (id) => set({ activeTabId: id }),

      updateTab: (id, updates) =>
        set((s) => ({
          tabs: s.tabs.map((t) => (t.id === id ? { ...t, ...updates } : t)),
        })),

      isDownloadPanelOpen: false,
      selectedDownloadIds: new Set(),

      toggleDownloadPanel: () => set((s) => ({ isDownloadPanelOpen: !s.isDownloadPanelOpen })),
      openDownloadPanel: () => set({ isDownloadPanelOpen: true }),
      closeDownloadPanel: () => set({ isDownloadPanelOpen: false }),

      toggleSelectDownload: (id) =>
        set((s) => {
          const next = new Set(s.selectedDownloadIds);
          if (next.has(id)) {
            next.delete(id);
          } else {
            next.add(id);
          }
          return { selectedDownloadIds: next };
        }),

      selectAll: (ids) => set({ selectedDownloadIds: new Set(ids) }),

      clearSelection: () => set({ selectedDownloadIds: new Set() }),

      // --- Settings ---
      autoImport: true,
      skipGamePicker: true,
      allowedExtensions: ['.zip', '.7z', '.rar', '.tar', '.gz'],
      retentionDays: 30,
      downloadsRoot: '',

      setAutoImport: (val) => set({ autoImport: val }),
      setSkipGamePicker: (val) => set({ skipGamePicker: val }),
      setAllowedExtensions: (exts) => set({ allowedExtensions: exts }),
      setRetentionDays: (days) => set({ retentionDays: days }),
      setDownloadsRoot: (path) => set({ downloadsRoot: path }),
    }),
    {
      name: 'emmm2-browser-store',
      partialize: (state) => ({
        tabs: state.tabs,
        activeTabId: state.activeTabId,
        autoImport: state.autoImport,
        skipGamePicker: state.skipGamePicker,
        allowedExtensions: state.allowedExtensions,
        retentionDays: state.retentionDays,
        downloadsRoot: state.downloadsRoot,
      }),
    },
  ),
);
