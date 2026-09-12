import { create } from 'zustand';

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
  /** Whether the app-level download confirmation modal is open. */
  isDownloadConfirmationOpen: boolean;

  toggleDownloadPanel: () => void;
  openDownloadPanel: () => void;
  closeDownloadPanel: () => void;
  setDownloadConfirmationOpen: (open: boolean) => void;

}

export const useBrowserStore = create<BrowserStore>()((set) => ({
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
      isDownloadConfirmationOpen: false,

      toggleDownloadPanel: () => set((s) => ({ isDownloadPanelOpen: !s.isDownloadPanelOpen })),
      openDownloadPanel: () => set({ isDownloadPanelOpen: true }),
      closeDownloadPanel: () => set({ isDownloadPanelOpen: false }),
      setDownloadConfirmationOpen: (open) => set({ isDownloadConfirmationOpen: open }),
    }));
