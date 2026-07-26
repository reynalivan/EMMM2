import { collectionKeys, corridorKeys } from '../../features/collections/queryKeys';
import { commands } from '../../lib/bindings';
import { queryClient } from '../../lib/queryClient';
import type { AppSliceCreator } from './sliceTypes';

export interface GameSlice {
  // Global Settings (Persisted in config.json)
  activeGameId: string | null;
  safeMode: boolean;
  autoCloseLauncher: boolean;
  isStoreInitialized: boolean;
  theme: 'onyx' | 'light';

  // Disk Reconcile bookkeeping
  lastDiskReconcileAtByGame: Record<string, number>;
  pendingDiskReconcileByGame: Record<string, boolean>;
  diskSourceUnavailableByGame: Record<string, string | null>;

  initStore: () => Promise<void>;
  setActiveGameId: (id: string | null) => Promise<void>;
  setAutoCloseLauncher: (enabled: boolean) => Promise<void>;
  setTheme: (theme: 'onyx' | 'light') => void;
  setDiskReconcileTimestamp: (gameId: string, timestamp: number) => void;
  markDiskReconcilePending: (gameId: string, dirty: boolean) => void;
  setDiskSourceUnavailable: (gameId: string, message: string | null) => void;
}

export const createGameSlice: AppSliceCreator<GameSlice> = (set) => ({
  activeGameId: null,
  safeMode: true,
  autoCloseLauncher: false,
  isStoreInitialized: false,
  theme: 'onyx',

  lastDiskReconcileAtByGame: {},
  pendingDiskReconcileByGame: {},
  diskSourceUnavailableByGame: {},

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
        await Promise.all([
          queryClient.prefetchQuery({
            queryKey: corridorKeys.state(settings.active_game_id),
            queryFn: () => commands.getCorridorState(settings.active_game_id as string, null),
          }),
          queryClient.prefetchQuery({
            queryKey: collectionKeys.list(settings.active_game_id),
            queryFn: () => commands.listCollections(settings.active_game_id as string, null),
          }),
        ]);
      }
    } catch (err) {
      console.error('Failed to init store from backend:', err);
      set({ isStoreInitialized: true });
    }
  },

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
      await commands.setActiveGame(id);
      if (id) {
        await Promise.all([
          queryClient.prefetchQuery({
            queryKey: corridorKeys.state(id),
            queryFn: () => commands.getCorridorState(id as string, null),
          }),
          queryClient.prefetchQuery({
            queryKey: collectionKeys.list(id),
            queryFn: () => commands.listCollections(id as string, null),
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
      await commands.setAutoCloseLauncher(enabled);
    } catch (e) {
      console.error('Failed to sync auto close launcher to backend', e);
    }
  },

  setTheme: (theme) => set({ theme }),

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
});
