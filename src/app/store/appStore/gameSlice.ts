import { collectionKeys, collectionRuntimeKeys } from '@/entities/collection';
import { listen } from '@tauri-apps/api/event';
import { commands } from '@/shared/api/tauri/bindings';
import { formatAppError } from '@/shared/lib/appError';
import { queryClient } from '@/shared/lib/queryClient';
import { settingsKeys } from '@/entities/settings';
import { toast } from '@/shared/ui/toast';
import type { AppSliceCreator } from './sliceTypes';
import type {
  DiskReconcileResult,
  DiskReconcileProgress,
  DiskReconcileReason,
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
  /** Monotonic backend revision of the newest disk observation for this game. */
  revision: number;
}

const EMPTY_DISK_RECONCILE: DiskReconcileEntry = {
  at: 0,
  pending: false,
  unavailable: null,
  progress: null,
  revision: 0,
};

export type FolderConflictReportStatus = 'open' | 'resolvedExternally' | 'cleared';

/** Latest disk-authoritative folder-conflict report for one game. */
export interface FolderConflictReport {
  revision: number;
  groups: FolderNameConflictGroup[];
  status: FolderConflictReportStatus;
  reason: DiskReconcileReason;
}

export interface GameSlice {
  // Global Settings (Persisted in config.json)
  activeGameId: string | null;
  autoCloseLauncher: boolean;

  // One entry per game so the three fields can never drift apart.
  diskReconcileByGame: Record<string, DiskReconcileEntry>;
  folderConflictsByGame: Record<string, FolderNameConflictGroup[]>;
  folderConflictReportsByGame: Record<string, FolderConflictReport>;
  renameConfirmationsByGame: Record<string, RenameConfirmationGroup[]>;

  initStore: () => Promise<void>;
  setActiveGameId: (id: string | null) => Promise<void>;
  setAutoCloseLauncher: (enabled: boolean) => Promise<void>;
  setDiskReconcileTimestamp: (gameId: string, timestamp: number) => void;
  setDiskReconcileProgress: (gameId: string, progress: DiskReconcileProgress | null) => void;
  markDiskReconcilePending: (gameId: string, dirty: boolean) => void;
  setDiskSourceUnavailable: (gameId: string, message: string | null) => void;
  setFolderConflicts: (gameId: string, conflicts: FolderNameConflictGroup[]) => void;
  applyFolderConflictReconcileResult: (result: DiskReconcileResult) => boolean;
  setRenameConfirmations: (gameId: string, groups: RenameConfirmationGroup[]) => void;
}

export const createGameSlice: AppSliceCreator<GameSlice> = (set, get) => ({
  activeGameId: null,
  autoCloseLauncher: false,

  diskReconcileByGame: {},
  folderConflictsByGame: {},
  folderConflictReportsByGame: {},
  renameConfirmationsByGame: {},

  initStore: async () => {
    const startupReportsByGame = new Map<string, DiskReconcileResult>();
    let startupInitialized = false;
    const unlisten = await listen<DiskReconcileResult>('disk_reconcile:result', (event) => {
      if (startupInitialized) {
        get().applyFolderConflictReconcileResult(event.payload);
        return;
      }

      const current = startupReportsByGame.get(event.payload.game_id);
      if (!current || event.payload.reconcile_revision > current.reconcile_revision) {
        startupReportsByGame.set(event.payload.game_id, event.payload);
      }
    }).catch(() => null);
    try {
      const settings = await commands.getSettings();
      const activeGameId = settings.active_game_id;
      const activeReport = activeGameId ? (startupReportsByGame.get(activeGameId) ?? null) : null;
      const activeReportRevision = activeReport?.reconcile_revision ?? 0;

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
                  revision: activeReportRevision,
                },
              },
              folderConflictsByGame: {
                [activeGameId]: activeReport.folder_conflicts,
              },
              folderConflictReportsByGame: {
                [activeGameId]: {
                  revision: activeReportRevision,
                  groups: activeReport.folder_conflicts,
                  status:
                    activeReport.status === 'AppliedWithFolderConflicts' &&
                    activeReport.folder_conflicts.length > 0
                      ? 'open'
                      : 'cleared',
                  reason: activeReport.reason,
                },
              },
              renameConfirmationsByGame: {
                [activeGameId]: activeReport.rename_confirmations,
              },
            }
          : {}),
      });
      startupInitialized = true;
      if (activeGameId) {
        await Promise.all([
          queryClient.prefetchQuery({
            queryKey: collectionRuntimeKeys.descriptor(activeGameId),
            queryFn: () => commands.getCollectionRuntimeDescriptor(activeGameId),
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
            queryKey: collectionRuntimeKeys.descriptor(id),
            queryFn: () => commands.getCollectionRuntimeDescriptor(id as string),
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

  // A successful reconcile resets volatile state while retaining its ordering revision.
  setDiskReconcileTimestamp: (gameId, timestamp) =>
    set((state) => {
      const current = state.diskReconcileByGame[gameId] ?? EMPTY_DISK_RECONCILE;
      return {
        diskReconcileByGame: {
          ...state.diskReconcileByGame,
          [gameId]: {
            ...current,
            at: timestamp,
            pending: false,
            unavailable: null,
            progress: null,
          },
        },
      };
    }),
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
  applyFolderConflictReconcileResult: (result) => {
    let applied = false;
    set((state) => {
      const diskState = state.diskReconcileByGame[result.game_id] ?? EMPTY_DISK_RECONCILE;
      if (result.reconcile_revision <= diskState.revision) {
        return state;
      }

      const currentReport = state.folderConflictReportsByGame[result.game_id];
      const hasFolderConflicts =
        result.status === 'AppliedWithFolderConflicts' && result.folder_conflicts.length > 0;
      const preserveOpenConflictReport =
        result.status === 'SourceUnavailable' && currentReport?.status === 'open';
      const verifiedNoFolderConflicts =
        result.status === 'Applied' || result.status === 'NeedsRenameConfirmation';
      let report: FolderConflictReport;
      if (preserveOpenConflictReport && currentReport) {
        report = currentReport;
      } else {
        const status: FolderConflictReportStatus = hasFolderConflicts
          ? 'open'
          : verifiedNoFolderConflicts &&
              (currentReport?.status === 'open' || currentReport?.status === 'resolvedExternally')
            ? 'resolvedExternally'
            : 'cleared';
        report = {
          revision: result.reconcile_revision,
          groups: hasFolderConflicts ? result.folder_conflicts : [],
          status,
          reason: result.reason,
        };
      }
      applied = true;
      return {
        diskReconcileByGame: {
          ...state.diskReconcileByGame,
          [result.game_id]: {
            ...diskState,
            revision: result.reconcile_revision,
          },
        },
        folderConflictsByGame: {
          ...state.folderConflictsByGame,
          [result.game_id]: report.groups,
        },
        folderConflictReportsByGame: {
          ...state.folderConflictReportsByGame,
          [result.game_id]: report,
        },
      };
    });
    return applied;
  },
  setRenameConfirmations: (gameId, groups) =>
    set((state) => ({
      renameConfirmationsByGame: {
        ...state.renameConfirmationsByGame,
        [gameId]: groups,
      },
    })),
});
