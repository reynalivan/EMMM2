import { collectionKeys, corridorKeys } from '../../features/collections/queryKeys';
import { commands } from '../../lib/bindings';
import { queryClient } from '../../lib/queryClient';
import type { AppSliceCreator } from './sliceTypes';

/** Disk Reconcile bookkeeping for one game. */
export interface DiskReconcileEntry {
  /** Epoch ms of the last successful reconcile. */
  at: number;
  /** Disk changed since that reconcile, so the next read must re-sync. */
  pending: boolean;
  /** Non-null while the mods folder is gone; the message explains why. */
  unavailable: string | null;
}

const EMPTY_DISK_RECONCILE: DiskReconcileEntry = { at: 0, pending: false, unavailable: null };

export interface GameSlice {
  // Global Settings (Persisted in config.json)
  activeGameId: string | null;
  autoCloseLauncher: boolean;

  // One entry per game so the three fields can never drift apart.
  diskReconcileByGame: Record<string, DiskReconcileEntry>;

  initStore: () => Promise<void>;
  setActiveGameId: (id: string | null) => Promise<void>;
  setAutoCloseLauncher: (enabled: boolean) => Promise<void>;
  setDiskReconcileTimestamp: (gameId: string, timestamp: number) => void;
  markDiskReconcilePending: (gameId: string, dirty: boolean) => void;
  setDiskSourceUnavailable: (gameId: string, message: string | null) => void;
}

export const createGameSlice: AppSliceCreator<GameSlice> = (set) => ({
  activeGameId: null,
  autoCloseLauncher: false,

  diskReconcileByGame: {},

  initStore: async () => {
    try {
      const settings = await commands.getSettings();

      set({
        activeGameId: settings.active_game_id,
        autoCloseLauncher: settings.auto_close_launcher ?? false,
      });

      if (settings.active_game_id) {
        await Promise.all([
          queryClient.prefetchQuery({
            queryKey: corridorKeys.state(settings.active_game_id),
            queryFn: () => commands.getCorridorState(settings.active_game_id as string),
          }),
          queryClient.prefetchQuery({
            queryKey: collectionKeys.list(settings.active_game_id),
            queryFn: () => commands.listCollections(settings.active_game_id as string),
          }),
        ]);
      }
    } catch (err) {
      console.error('Failed to init store from backend:', err);
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
            queryFn: () => commands.getCorridorState(id as string),
          }),
          queryClient.prefetchQuery({
            queryKey: collectionKeys.list(id),
            queryFn: () => commands.listCollections(id as string),
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

  // A successful reconcile resets the whole entry, so it replaces rather than patches.
  setDiskReconcileTimestamp: (gameId, timestamp) =>
    set((state) => ({
      diskReconcileByGame: {
        ...state.diskReconcileByGame,
        [gameId]: { at: timestamp, pending: false, unavailable: null },
      },
    })),
  markDiskReconcilePending: (gameId, dirty) =>
    set((state) => ({
      diskReconcileByGame: {
        ...state.diskReconcileByGame,
        [gameId]: {
          ...(state.diskReconcileByGame[gameId] ?? EMPTY_DISK_RECONCILE),
          pending: dirty,
        },
      },
    })),
  setDiskSourceUnavailable: (gameId, message) =>
    set((state) => ({
      diskReconcileByGame: {
        ...state.diskReconcileByGame,
        [gameId]: {
          ...(state.diskReconcileByGame[gameId] ?? EMPTY_DISK_RECONCILE),
          unavailable: message,
          pending: false,
        },
      },
    })),
});
