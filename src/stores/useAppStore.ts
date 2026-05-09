import { create } from 'zustand';
import { persist } from 'zustand/middleware';

import type { SortField, SortOrder, ViewMode } from '../types/mod';
import { collectionKeys, corridorKeys } from '../features/collections/queryKeys';
import { areObjectMetaFiltersEqual } from '../features/object-list/objectFilterState';

import { commands } from '../lib/bindings';
import { queryClient } from '../lib/queryClient';
import type {
  WorkspaceDialogState,
  WorkspacePreviewTransitionState,
} from '../features/workspace-runtime/state/workspaceState';
type WorkspaceView =
  | 'dashboard'
  | 'mods'
  | 'collections'
  | 'settings'
  | 'browser'
  | 'downloads'
  | 'storage-optimizer';
type MobilePane = 'sidebar' | 'grid' | 'details';

/** Centralized safe mode toggle flow state — prevents race conditions from multiple hook instances. */
export type SafeModeFlowState =
  | { kind: 'idle' }
  | { kind: 'pin' }
  | { kind: 'confirm'; targetSafe: boolean };

export interface AppState {
  // Global Settings (Persisted in config.json)
  activeGameId: string | null;
  safeMode: boolean;
  autoCloseLauncher: boolean;
  isStoreInitialized: boolean;
  theme: 'onyx' | 'light';

  // Navigation State
  workspaceView: WorkspaceView;
  currentPath: string[];

  // Mobile Navigation State
  mobileActivePane: MobilePane;

  // Desktop Layout State
  isPreviewOpen: boolean;

  // Selection State
  selectedObjectFolderPath: string | null;
  selectedModPath: string | null;
  gridSelection: Set<string>;
  workspacePreviewDirty: boolean;
  workspacePreviewTransition: WorkspacePreviewTransitionState;
  workspaceDialogState: WorkspaceDialogState;

  // Epic 3: Sidebar State
  selectedObjectType: string | null;
  sidebarSearchQuery: string;
  collapsedCategories: Set<string>;
  objectMetaFilters: Record<string, string[]>;
  objectSortBy: 'name' | 'date' | 'rarity';
  objectStatusFilter: 'all' | 'enabled' | 'disabled';

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

  // Disk Reconcile bookkeeping
  lastDiskReconcileAtByGame: Record<string, number>;
  pendingDiskReconcileByGame: Record<string, boolean>;
  diskSourceUnavailableByGame: Record<string, string | null>;
  setDiskReconcileTimestamp: (gameId: string, timestamp: number) => void;
  markDiskReconcilePending: (gameId: string, dirty: boolean) => void;
  setDiskSourceUnavailable: (gameId: string, message: string | null) => void;

  // Context-Aware Selection
  activePane: 'objectList' | 'folderGrid';
  setActivePane: (pane: 'objectList' | 'folderGrid') => void;

  // Safe Mode Flow (centralized to prevent concurrent-instance race conditions)
  safeModeFlow: SafeModeFlowState;
  setSafeModeFlow: (flow: SafeModeFlowState) => void;

  // Ignore Management State
  isIgnoreManagementOpen: boolean;
  setIgnoreManagementOpen: (open: boolean) => void;

  // Actions
  initStore: () => Promise<void>;
  setActiveGameId: (id: string | null) => Promise<void>;

  setAutoCloseLauncher: (enabled: boolean) => Promise<void>;

  setWorkspaceView: (view: WorkspaceView) => void;
  setCurrentPath: (path: string[]) => void;
  setSelectedObjectFolderPath: (folderPath: string | null) => void;
  setSelectedModPath: (path: string | null) => void;

  toggleGridSelection: (id: string, multi?: boolean) => void;
  clearGridSelection: () => void;
  setGridSelection: (selection: Set<string>) => void;
  replaceGridSelection: (oldPath: string, newPath: string) => void;
  setPanelWidths: (left: number, right: number) => void;

  // Responsive Actions
  setMobilePane: (pane: MobilePane) => void;
  togglePreview: () => void;

  // Epic 3: Sidebar Actions
  setSelectedObjectType: (type: string | null) => void;
  setSidebarSearch: (query: string) => void;
  toggleCategoryCollapse: (category: string) => void;
  setObjectMetaFilters: (filters: Record<string, string[]>) => void;
  setObjectSortBy: (sortBy: 'name' | 'date' | 'rarity') => void;
  setObjectStatusFilter: (filter: 'all' | 'enabled' | 'disabled') => void;

  // Epic 4: Explorer Actions
  setSortField: (field: SortField) => void;
  setSortOrder: (order: SortOrder) => void;
  setViewMode: (mode: ViewMode) => void;
  setExplorerSubPath: (subPath: string | undefined) => void;
  setExplorerSearch: (query: string) => void;
  setExplorerScrollOffset: (offset: number) => void;
  setTheme: (theme: 'onyx' | 'light') => void;
  correctExplorerPath: (oldPath: string, newPath: string) => void;
}

import { createJSONStorage } from 'zustand/middleware';

// Custom debounced storage to prevent LocalStorage spam
const debouncedStorage = {
  getItem: (name: string) => {
    return localStorage.getItem(name);
  },
  setItem: (() => {
    let timeoutId: number | null = null;
    return (name: string, value: string) => {
      if (timeoutId) window.clearTimeout(timeoutId);
      timeoutId = window.setTimeout(() => {
        localStorage.setItem(name, value);
      }, 300); // 300ms debounce
    };
  })(),
  removeItem: (name: string) => {
    localStorage.removeItem(name);
  },
};

export const useAppStore = create<AppState>()(
  persist(
    (set, get) => ({
      // Defaults
      activeGameId: null,
      safeMode: true,
      autoCloseLauncher: false,
      isStoreInitialized: false,
      theme: 'onyx',
      workspaceView: 'dashboard',
      currentPath: [],
      selectedObjectFolderPath: null,
      selectedModPath: null,
      gridSelection: new Set(),
      workspacePreviewDirty: false,
      workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
      workspaceDialogState: { kind: 'none' },

      leftPanelWidth: 260,
      rightPanelWidth: 320,

      // Epic 3: Sidebar Defaults
      selectedObjectType: null,
      sidebarSearchQuery: '',
      collapsedCategories: new Set(),
      objectMetaFilters: {},
      objectSortBy: 'name',
      objectStatusFilter: 'all',

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

      // Safe Mode Flow
      safeModeFlow: { kind: 'idle' } as SafeModeFlowState,
      setSafeModeFlow: (flow) => set({ safeModeFlow: flow }),

      // Ignore Management
      isIgnoreManagementOpen: false,
      setIgnoreManagementOpen: (open) => set({ isIgnoreManagementOpen: open }),

      // Disk Reconcile bookkeeping
      lastDiskReconcileAtByGame: {},
      pendingDiskReconcileByGame: {},
      diskSourceUnavailableByGame: {},
      setDiskReconcileTimestamp: (gameId, timestamp) =>
        set((state) => ({
          lastDiskReconcileAtByGame: {
            ...state.lastDiskReconcileAtByGame,
            [gameId]: timestamp,
          },
          pendingDiskReconcileByGame: {
            ...state.pendingDiskReconcileByGame,
            [gameId]: false,
          },
          diskSourceUnavailableByGame: {
            ...state.diskSourceUnavailableByGame,
            [gameId]: null,
          },
        })),
      markDiskReconcilePending: (gameId, dirty) =>
        set((state) => ({
          pendingDiskReconcileByGame: {
            ...state.pendingDiskReconcileByGame,
            [gameId]: dirty,
          },
        })),
      setDiskSourceUnavailable: (gameId, message) =>
        set((state) => ({
          diskSourceUnavailableByGame: {
            ...state.diskSourceUnavailableByGame,
            [gameId]: message,
          },
          pendingDiskReconcileByGame: {
            ...state.pendingDiskReconcileByGame,
            [gameId]: false,
          },
        })),

      // Context-Aware Selection
      activePane: 'objectList',
      setActivePane: (pane) => set({ activePane: pane }),

      // Store Initialization
      initStore: async () => {
        try {
          const settings = await commands.getSettings();

          set({
            activeGameId: settings.active_game_id,
            safeMode: settings.safe_mode.enabled ?? false,
            autoCloseLauncher: settings.auto_close_launcher ?? false,
            isStoreInitialized: true,
          });

          if (settings.active_game_id) {
            const safeMode = settings.safe_mode.enabled ?? false;
            await Promise.all([
              queryClient.prefetchQuery({
                queryKey: corridorKeys.state(settings.active_game_id, safeMode),
                queryFn: () =>
                  commands.getCorridorState({
                    gameId: settings.active_game_id as string,
                    isSafe: safeMode,
                  }),
              }),
              queryClient.prefetchQuery({
                queryKey: collectionKeys.list(settings.active_game_id, safeMode),
                queryFn: () =>
                  commands.listCollections({
                    gameId: settings.active_game_id as string,
                    isSafe: safeMode,
                  }),
              }),
            ]);
          }
        } catch (err) {
          console.error('Failed to init store from backend:', err);
          set({ isStoreInitialized: true });
        }
      },

      // Actions
      setActiveGameId: async (id) => {
        set({
          activeGameId: id,
          // Reset explorer state to prevent stale paths from previous game
          explorerSubPath: undefined,
          currentPath: [],
          explorerSearchQuery: '',
          selectedObjectFolderPath: null,
          selectedModPath: null,
          gridSelection: new Set(),
          workspacePreviewDirty: false,
          workspacePreviewTransition: { kind: 'idle', pendingTarget: null },
          workspaceDialogState: { kind: 'none' },
          // Reset sidebar state to prevent stale filters from previous game
          sidebarSearchQuery: '',
          selectedObjectType: null,
          collapsedCategories: new Set(),
          objectMetaFilters: {},
          objectSortBy: 'name',
          objectStatusFilter: 'all',
        });

        try {
          await commands.setActiveGame({ gameId: id });
          if (id) {
            const safeMode = get().safeMode;
            await Promise.all([
              queryClient.prefetchQuery({
                queryKey: corridorKeys.state(id, safeMode),
                queryFn: () =>
                  commands.getCorridorState({
                    gameId: id as string,
                    isSafe: safeMode,
                  }),
              }),
              queryClient.prefetchQuery({
                queryKey: collectionKeys.list(id, safeMode),
                queryFn: () => commands.listCollections({ gameId: id as string, isSafe: safeMode }),
              }),
            ]);
          }
        } catch (e) {
          console.error('Failed to sync active game to backend', e);
        }
      },

      setAutoCloseLauncher: async (enabled) => {
        set({ autoCloseLauncher: enabled });
        try {
          // This saves the entire AppSettings backend representation since we don't have a
          // dedicated command for just autoCloseLauncher. It's safe to use `update_settings`
          // but if that command doesn't exist, we fallback.
          await commands.setAutoCloseLauncher({ enabled });
        } catch (e) {
          console.error('Failed to sync auto close launcher to backend', e);
        }
      },

      setWorkspaceView: (view) => set({ workspaceView: view }),
      setCurrentPath: (path) => set({ currentPath: path }),

      setSelectedObjectFolderPath: (folderPath) =>
        set({
          selectedObjectFolderPath: folderPath,
          // Auto-navigate to grid on mobile when object selected
          mobileActivePane: folderPath ? 'grid' : 'sidebar',
        }),
      setSelectedModPath: (path) => set({ selectedModPath: path }),

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
            selectedModPath: newSet.size > 0 ? id : null,
            mobileActivePane: nextMobilePane,
          };
        }),

      clearGridSelection: () => set({ gridSelection: new Set(), selectedModPath: null }),

      setGridSelection: (selection) =>
        set((state) => {
          // Auto-navigate to details on mobile when item selected (single select)
          const nextMobilePane = selection.size === 1 ? 'details' : state.mobileActivePane;
          const selectionEntries = Array.from(selection);
          return {
            gridSelection: selection,
            selectedModPath:
              selectionEntries.length > 0 ? selectionEntries[selectionEntries.length - 1] : null,
            mobileActivePane: nextMobilePane,
          };
        }),

      replaceGridSelection: (oldPath, newPath) =>
        set((state) => {
          if (!state.gridSelection.has(oldPath)) return state;
          const newSet = new Set(state.gridSelection);
          newSet.delete(oldPath);
          newSet.add(newPath);
          return {
            gridSelection: newSet,
            selectedModPath: state.selectedModPath === oldPath ? newPath : state.selectedModPath,
          };
        }),

      setPanelWidths: (left, right) => set({ leftPanelWidth: left, rightPanelWidth: right }),

      setMobilePane: (pane) => set({ mobileActivePane: pane }),
      togglePreview: () => set((state) => ({ isPreviewOpen: !state.isPreviewOpen })),

      // Epic 3: Sidebar Actions
      setSelectedObjectType: (type) =>
        set((state) => (state.selectedObjectType === type ? state : { selectedObjectType: type })),
      setSidebarSearch: (query) =>
        set((state) =>
          state.sidebarSearchQuery === query ? state : { sidebarSearchQuery: query },
        ),
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
      setObjectMetaFilters: (filters) =>
        set((state) =>
          areObjectMetaFiltersEqual(state.objectMetaFilters, filters)
            ? state
            : { objectMetaFilters: filters },
        ),
      setObjectSortBy: (sortBy) =>
        set((state) => (state.objectSortBy === sortBy ? state : { objectSortBy: sortBy })),
      setObjectStatusFilter: (filter) =>
        set((state) =>
          state.objectStatusFilter === filter ? state : { objectStatusFilter: filter },
        ),

      // Epic 4: Explorer Actions
      setSortField: (field) => set({ sortField: field }),
      setSortOrder: (order) => set({ sortOrder: order }),
      setViewMode: (mode) => set({ viewMode: mode }),
      setExplorerSubPath: (subPath) => set({ explorerSubPath: subPath }),
      setExplorerSearch: (query) => set({ explorerSearchQuery: query }),
      setExplorerScrollOffset: (offset) => set({ explorerScrollOffset: offset }),
      setTheme: (theme) => set({ theme }),
      correctExplorerPath: (oldPath, newPath) => {
        const oldName = oldPath.split(/[/\\]/).pop() || '';
        const newName = newPath.split(/[/\\]/).pop() || '';

        set((state) => {
          const normalize = (p: string) => p.replace(/\\/g, '/');
          const normOldName = normalize(oldName);
          const normNewName = normalize(newName);

          let nextSubPath = state.explorerSubPath;
          if (state.explorerSubPath) {
            const normSub = normalize(state.explorerSubPath);
            // Case 1: subPath is exactly the renamed folder
            if (normSub === normOldName) {
              nextSubPath = normNewName;
            }
            // Case 2: subPath is a descendant of the renamed folder
            else if (normSub.startsWith(normOldName + '/')) {
              nextSubPath = normNewName + normSub.substring(normOldName.length);
            }
          }

          // Update breadcrumbs (array of names)
          const nextCurrentPath = state.currentPath.map((p) => (p === oldName ? newName : p));

          // Update selection (absolute path)
          let nextSelectedFolder = state.selectedObjectFolderPath;
          if (state.selectedObjectFolderPath === oldPath) {
            nextSelectedFolder = newPath;
          } else if (
            state.selectedObjectFolderPath?.startsWith(oldPath + '/') ||
            state.selectedObjectFolderPath?.startsWith(oldPath + '\\')
          ) {
            nextSelectedFolder = state.selectedObjectFolderPath.replace(oldPath, newPath);
          }

          return {
            explorerSubPath: nextSubPath,
            currentPath: nextCurrentPath,
            selectedObjectFolderPath: nextSelectedFolder,
          };
        });
      },
    }),
    {
      name: 'vibecode-storage',
      storage: createJSONStorage(() => debouncedStorage),
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
        theme: state.theme,

        // Epic 3: Persist collapsed categories (serializable array)
        collapsedCategories: Array.from(state.collapsedCategories),
      }),
      merge: (persistedState: unknown, currentState) => {
        const pState = persistedState as Partial<AppState>;

        return {
          ...currentState,
          leftPanelWidth: pState.leftPanelWidth ?? currentState.leftPanelWidth,
          rightPanelWidth: pState.rightPanelWidth ?? currentState.rightPanelWidth,
          isPreviewOpen: pState.isPreviewOpen ?? currentState.isPreviewOpen,
          sortField: pState.sortField ?? currentState.sortField,
          sortOrder: pState.sortOrder ?? currentState.sortOrder,
          viewMode: pState.viewMode ?? currentState.viewMode,
          currentPath: pState.currentPath ?? currentState.currentPath,
          explorerSubPath: pState.explorerSubPath ?? currentState.explorerSubPath,
          explorerScrollOffset: pState.explorerScrollOffset ?? currentState.explorerScrollOffset,
          theme: pState.theme ?? currentState.theme,
          // Deserialize array back to Set when loading
          collapsedCategories: pState?.collapsedCategories
            ? new Set(pState.collapsedCategories)
            : currentState.collapsedCategories,
        };
      },
    },
  ),
);
