import { create } from 'zustand';
import { persist } from 'zustand/middleware';

import type { SortField, SortOrder, ViewMode } from '../types/mod';

type WorkspaceView = 'dashboard' | 'mods' | 'collections' | 'settings';
type MobilePane = 'sidebar' | 'grid' | 'details';

interface AppState {
  // Global Settings (Persisted in config.json)
  activeGameId: string | null;
  safeMode: boolean;
  isStoreInitialized: boolean;

  // Navigation State
  workspaceView: WorkspaceView;
  currentPath: string[];

  // Mobile Navigation State
  mobileActivePane: MobilePane;

  // Desktop Layout State
  isPreviewOpen: boolean;

  // Selection State
  activeCollectionId: string | null;
  selectedObject: string | null;
  gridSelection: Set<string>;

  // Epic 3: Sidebar State
  selectedObjectType: string | null;
  sidebarSearchQuery: string;
  collapsedCategories: Set<string>;

  // Layout State (Persisted in LocalStorage via Zustand)
  leftPanelWidth: number;
  rightPanelWidth: number;

  // Epic 4: Explorer State
  sortField: SortField;
  sortOrder: SortOrder;
  viewMode: ViewMode;
  explorerSubPath: string | undefined;
  explorerSearchQuery: string;
  explorerScrollOffset: number;

  // Actions
  initStore: () => Promise<void>;
  setActiveGameId: (id: string | null) => Promise<void>;
  setSafeMode: (enabled: boolean) => Promise<void>;

  setWorkspaceView: (view: WorkspaceView) => void;
  setCurrentPath: (path: string[]) => void;
  setActiveCollectionId: (id: string | null) => void;
  setSelectedObject: (id: string | null) => void;
  toggleGridSelection: (id: string, multi?: boolean) => void;
  clearGridSelection: () => void;
  setPanelWidths: (left: number, right: number) => void;

  // Responsive Actions
  setMobilePane: (pane: MobilePane) => void;
  togglePreview: () => void;

  // Epic 3: Sidebar Actions
  setSelectedObjectType: (type: string | null) => void;
  setSidebarSearch: (query: string) => void;
  toggleCategoryCollapse: (category: string) => void;

  // Epic 4: Explorer Actions
  setSortField: (field: SortField) => void;
  setSortOrder: (order: SortOrder) => void;
  setViewMode: (mode: ViewMode) => void;
  setExplorerSubPath: (subPath: string | undefined) => void;
  setExplorerSearch: (query: string) => void;
  setExplorerScrollOffset: (offset: number) => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      // Defaults
      activeGameId: null,
      safeMode: true,
      isStoreInitialized: false,
      workspaceView: 'dashboard',
      currentPath: [],
      activeCollectionId: null,
      selectedObject: null,
      gridSelection: new Set(),
      leftPanelWidth: 260,
      rightPanelWidth: 320,

      // Epic 3: Sidebar Defaults
      selectedObjectType: null,
      sidebarSearchQuery: '',
      collapsedCategories: new Set(),

      // Responsive Defaults
      mobileActivePane: 'sidebar',
      isPreviewOpen: true,

      // Epic 4: Explorer Defaults
      sortField: 'name',
      sortOrder: 'asc',
      viewMode: 'grid',
      explorerSubPath: undefined,
      explorerSearchQuery: '',
      explorerScrollOffset: 0,

      // Store Initialization
      initStore: async () => {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          // Fetch full settings from backend (source of truth)
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          const settings = await invoke<Record<string, any>>('get_settings');

          set({
            activeGameId: settings.active_game_id,
            safeMode: settings.safe_mode.enabled,
            isStoreInitialized: true,
          });
        } catch (err) {
          console.error('Failed to init store from backend:', err);
          set({ isStoreInitialized: true });
        }
      },

      // Actions
      setActiveGameId: async (id) => {
        const { invoke } = await import('@tauri-apps/api/core');
        set({
          activeGameId: id,
          // Reset explorer state to prevent stale paths from previous game
          explorerSubPath: undefined,
          currentPath: [],
          explorerSearchQuery: '',
          selectedObject: null,
          gridSelection: new Set(),
          // Reset sidebar state to prevent stale filters from previous game
          sidebarSearchQuery: '',
          selectedObjectType: null,
          collapsedCategories: new Set(),
        });

        try {
          await invoke('set_active_game', { gameId: id });
        } catch (e) {
          console.error('Failed to sync active game to backend', e);
        }
      },

      setSafeMode: async (enabled) => {
        const { invoke } = await import('@tauri-apps/api/core');
        set({ safeMode: enabled });
        try {
          await invoke('set_safe_mode_enabled', { enabled });
        } catch (e) {
          console.error('Failed to sync safe mode to backend', e);
        }
      },

      setWorkspaceView: (view) => set({ workspaceView: view }),
      setCurrentPath: (path) => set({ currentPath: path }),
      setActiveCollectionId: (id) => set({ activeCollectionId: id }),

      setSelectedObject: (id) =>
        set({
          selectedObject: id,
          // Auto-navigate to grid on mobile when object selected
          mobileActivePane: id ? 'grid' : 'sidebar',
        }),

      toggleGridSelection: (id, multi = false) =>
        set((state) => {
          const newSet = new Set(multi ? state.gridSelection : []);
          if (newSet.has(id)) {
            newSet.delete(id);
          } else {
            newSet.add(id);
          }

          // Auto-navigate to details on mobile when item selected (single select)
          const nextMobilePane = newSet.size > 0 && !multi ? 'details' : state.mobileActivePane;

          return {
            gridSelection: newSet,
            mobileActivePane: nextMobilePane,
          };
        }),

      clearGridSelection: () => set({ gridSelection: new Set() }),

      setPanelWidths: (left, right) => set({ leftPanelWidth: left, rightPanelWidth: right }),

      setMobilePane: (pane) => set({ mobileActivePane: pane }),
      togglePreview: () => set((state) => ({ isPreviewOpen: !state.isPreviewOpen })),

      // Epic 3: Sidebar Actions
      setSelectedObjectType: (type) => set({ selectedObjectType: type }),
      setSidebarSearch: (query) => set({ sidebarSearchQuery: query }),
      toggleCategoryCollapse: (category) =>
        set((state) => {
          const next = new Set(state.collapsedCategories);
          if (next.has(category)) {
            next.delete(category);
          } else {
            next.add(category);
          }
          return { collapsedCategories: next };
        }),

      // Epic 4: Explorer Actions
      setSortField: (field) => set({ sortField: field }),
      setSortOrder: (order) => set({ sortOrder: order }),
      setViewMode: (mode) => set({ viewMode: mode }),
      setExplorerSubPath: (subPath) => set({ explorerSubPath: subPath }),
      setExplorerSearch: (query) => set({ explorerSearchQuery: query }),
      setExplorerScrollOffset: (offset) => set({ explorerScrollOffset: offset }),
    }),
    {
      name: 'vibecode-storage',
      partialize: (state) => ({
        leftPanelWidth: state.leftPanelWidth,
        rightPanelWidth: state.rightPanelWidth,
        isPreviewOpen: state.isPreviewOpen,
        // Epic 4: Persist explorer preferences
        sortField: state.sortField,
        sortOrder: state.sortOrder,
        viewMode: state.viewMode,
        currentPath: state.currentPath,
        explorerSubPath: state.explorerSubPath,
        explorerScrollOffset: state.explorerScrollOffset,
      }),
    },
  ),
);
