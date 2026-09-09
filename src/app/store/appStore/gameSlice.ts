import { collectionKeys, collectionRuntimeKeys } from '@/entities/collection';
import { listen } from '@tauri-apps/api/event';
import { commands } from '@/shared/api/tauri/bindings';
import { formatAppError } from '@/shared/lib/appError';
import { queryClient } from '@/shared/lib/queryClient';
import { settingsKeys } from '@/entities/settings';
import { toast } from '../useToastStore';
import type { AppSliceCreator } from './sliceTypes';
import type {
  DiskReconcileResult,
  DiskReconcileProgress,
  FolderNameConflictGroup,
  RenameConfirmationGroup,
} from '@/shared/api/tauri/bindings';

/** Disk Reconcile bookkeeping for one game. */
export interface DiskReconcileEntry {
  /** Epoch ms of the last successful reconcile. */
  at: number;
  /** Disk changed since that reconcile, so the next read must re-sync. */
  pending: boolean;
  /** Non-null while the mods folder is gone; the message explains why. */
  unavailable: string | null;
  /** Latest measured scan progress, cleared by every terminal result. */
  progress: DiskReconcileProgress | null;
}

const EMPTY_DISK_RECONCILE: DiskReconcileEntry = {
  at: 0,
  pending: false,
  unavailable: null,
  progress: null,
};

export interface GameSlice {
  // Global Settings (Persisted in config.json)
  activeGameId: string | null;
  autoCloseLauncher: boolean;

  // One entry per game so the three fields can never drift apart.
  diskReconcileByGame: Record<string, DiskReconcileEntry>;
  folderConflictsByGame: Record<string, FolderNameConflictGroup[]>;
  renameConfirmationsByGame: Record<string, RenameConfirmationGroup[]>;

  initStore: () => Promise<void>;
  setActiveGameId: (id: string | null) => Promise<void>;
  setAutoCloseLauncher: (enabled: boolean) => Promise<void>;
  setDiskReconcileTimestamp: (gameId: string, timestamp: number) => void;
  setDiskReconcileProgress: (gameId: string, progress: DiskReconcileProgress | null) => void;
  markDiskReconcilePending: (gameId: string, dirty: boolean) => void;
  setDiskSourceUnavailable: (gameId: string, message: string | null) => void;
  setFolderConflicts: (gameId: string, conflicts: FolderNameConflictGroup[]) => void;
  setRenameConfirmations: (gameId: string, groups: RenameConfirmationGroup[]) => void;
}

export const createGameSlice: AppSliceCreator<GameSlice> = (set, get) => ({
  activeGameId: null,
  autoCloseLauncher: false,

  diskReconcileByGame: {},
  folderConflictsByGame: {},
  renameConfirmationsByGame: {},

  initStore: async () => {
    const startupReport = { current: null as DiskReconcileResult | null };
    const unlisten = await listen<DiskReconcileResult>('disk_reconcile:result', (event) => {
      startupReport.current = event.payload;
    }).catch(() => null);
    try {
      const settings = await commands.getSettings();
      const activeGameId = settings.active_game_id;
      const report = startupReport.current;
      const activeReport = report?.game_id === activeGameId ? report : null;

      set({
        activeGameId,
        autoCloseLauncher: settings.auto_close_launcher ?? false,
        ...(activeGameId && activeReport
          ? {
              diskReconcileByGame: {
                [activeGameId]: {
                  at: activeReport.status === 'Applied' ? Date.now() : 0,
                  pending: false,
                  unavailable:
                    activeReport.status === 'SourceUnavailable'
                      ? (activeReport.error_message ?? 'Mods folder is unavailable')
                      : null,
                  progress: null,
                },
              },
              folderConflictsByGame: {
                [activeGameId]: activeReport.folder_conflicts,
              },
              renameConfirmationsByGame: {
                [activeGameId]: activeReport.rename_confirmations,
              },
            }
          : {}),
      });

      if (activeGameId) {
        await Promise.all([
          queryClient.prefetchQuery({
            queryKey: collectionRuntimeKeys.state(activeGameId),
            queryFn: () => commands.getCollectionRuntimeState(activeGameId),
          }),
          queryClient.prefetchQuery({
            queryKey: collectionKeys.list(activeGameId),
            queryFn: () => commands.listCollections(activeGameId),
          }),
        ]);
      }
    } catch (err) {
      console.error('Failed to init store from backend:', err);
      toast.error(formatAppError(err));
    } finally {
      unlisten?.();
    }
  },

  setActiveGameId: async (id) => {
    try {
      // The backend resets the per-game disk recovery gate here. Publish the
      // new active ID only afterwards so no workspace query can race ahead and
      // hydrate from a projection created before external/offline changes.
      await commands.setActiveGame(id);
      queryClient.setQueryData(settingsKeys.all, await commands.getSettings());
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
      if (id) {
        await Promise.all([
          queryClient.prefetchQuery({
            queryKey: collectionRuntimeKeys.state(id),
            queryFn: () => commands.getCollectionRuntimeState(id as string),
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
    const previous = get().autoCloseLauncher;
    set({ autoCloseLauncher: enabled });
    try {
      await commands.setAutoCloseLauncher(enabled);
      queryClient.setQueryData(settingsKeys.all, await commands.getSettings());
    } catch (e) {
      console.error('Failed to sync auto close launcher to backend', e);
      if (get().autoCloseLauncher === enabled) {
        set({ autoCloseLauncher: previous });
      }
      toast.error(formatAppError(e));
    }
  },

  // A successful reconcile resets the whole entry, so it replaces rather than patches.
  setDiskReconcileTimestamp: (gameId, timestamp) =>
    set((state) => ({
      diskReconcileByGame: {
        ...state.diskReconcileByGame,
        [gameId]: { at: timestamp, pending: false, unavailable: null, progress: null },
      },
    })),
  setDiskReconcileProgress: (gameId, progress) =>
    set((state) => ({
      diskReconcileByGame: {
        ...state.diskReconcileByGame,
        [gameId]: {
          ...(state.diskReconcileByGame[gameId] ?? EMPTY_DISK_RECONCILE),
          progress,
          pending: progress !== null,
        },
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
          progress: null,
        },
      },
    })),
  setFolderConflicts: (gameId, conflicts) =>
    set((state) => ({
      folderConflictsByGame: {
        ...state.folderConflictsByGame,
        [gameId]: conflicts,
      },
    })),
  setRenameConfirmations: (gameId, groups) =>
    set((state) => ({
      renameConfirmationsByGame: {
        ...state.renameConfirmationsByGame,
        [gameId]: groups,
      },
    })),
});
