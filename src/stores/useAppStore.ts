import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { configStore, saveConfig, StoreKeys } from '../lib/store';

type GameType = 'GIMI' | 'SRMI' | 'ZZMI';
type WorkspaceView = 'dashboard' | 'mods';
type MobilePane = 'sidebar' | 'grid' | 'details';

interface AppState {
  // Global Settings (Persisted in config.json)
  activeGame: GameType;
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
  activePreset: string | null;
  selectedObject: string | null;
  gridSelection: Set<string>;

  // Epic 3: Sidebar State
  selectedObjectType: string | null;
  sidebarSearchQuery: string;
  collapsedCategories: Set<string>;

  // Layout State (Persisted in LocalStorage via Zustand)
  leftPanelWidth: number;
  rightPanelWidth: number;

  // Actions
  initStore: () => Promise<void>;
  setActiveGame: (game: GameType) => Promise<void>;
  setSafeMode: (enabled: boolean) => Promise<void>;

  setWorkspaceView: (view: WorkspaceView) => void;
  setCurrentPath: (path: string[]) => void;
  setActivePreset: (preset: string | null) => void;
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
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      // Defaults
      activeGame: 'GIMI',
      safeMode: true,
      isStoreInitialized: false,
      workspaceView: 'dashboard',
      currentPath: [],
      activePreset: null,
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

      // Store Initialization
      initStore: async () => {
        try {
          const storedGame = await configStore.get<string>(StoreKeys.ACTIVE_GAME);
          const storedSafeMode = await configStore.get<boolean>(StoreKeys.SAFE_MODE);

          set({
            activeGame: (storedGame as GameType) || 'GIMI',
            safeMode: storedSafeMode ?? true,
            isStoreInitialized: true,
          });
        } catch (err) {
          console.error('Failed to init store:', err);
          set({ isStoreInitialized: true });
        }
      },

      // Actions
      setActiveGame: async (game) => {
        set({ activeGame: game });
        await saveConfig(StoreKeys.ACTIVE_GAME, game);
      },

      setSafeMode: async (enabled) => {
        set({ safeMode: enabled });
        await saveConfig(StoreKeys.SAFE_MODE, enabled);
      },

      setWorkspaceView: (view) => set({ workspaceView: view }),
      setCurrentPath: (path) => set({ currentPath: path }),
      setActivePreset: (preset) => set({ activePreset: preset }),

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
    }),
    {
      name: 'vibecode-storage',
      partialize: (state) => ({
        leftPanelWidth: state.leftPanelWidth,
        rightPanelWidth: state.rightPanelWidth,
        isPreviewOpen: state.isPreviewOpen, // Persist desktop preview state
      }),
    },
  ),
);
