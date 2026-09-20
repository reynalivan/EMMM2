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
  GameActivationStatus,
  RenameConfirmationGroup,
  RuntimeSyncStatus,
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

let runtimeStatusListenersReady: Promise<void> | null = null;
let activeGameRequestSequence = 0;

const runtimePhaseOrder: Record<RuntimeSyncStatus['phase'], number> = {
  queued: 0,
  running: 1,
  succeeded: 2,
  needs_manual_reload: 2,
  failed: 2,
};

const activationPhaseOrder: Record<GameActivationStatus['phase'], number> = {
  syncing: 0,
  ready: 1,
  source_unavailable: 1,
  failed: 1,
};

function activationStatusInformationScore(status: GameActivationStatus): number {
  return (
    Number(status.reconcile_revision !== null) +
    Number(status.runtime_sync_generation !== null) +
    Number(status.error !== null)
  );
}

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
  runtimeSyncByGame: Record<string, RuntimeSyncStatus>;
  gameActivationByGame: Record<string, GameActivationStatus>;

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
  setRuntimeSyncStatus: (status: RuntimeSyncStatus) => void;
  setGameActivationStatus: (status: GameActivationStatus) => void;
}

export const createGameSlice: AppSliceCreator<GameSlice> = (set, get) => ({
  activeGameId: null,
  autoCloseLauncher: false,

  diskReconcileByGame: {},
  folderConflictsByGame: {},
  folderConflictReportsByGame: {},
  renameConfirmationsByGame: {},
  runtimeSyncByGame: {},
  gameActivationByGame: {},

  initStore: async () => {
    if (!runtimeStatusListenersReady) {
      runtimeStatusListenersReady = Promise.all([
        listen<RuntimeSyncStatus>('runtime_sync:status', (event) => {
          get().setRuntimeSyncStatus(event.payload);
        }),
        listen<GameActivationStatus>('game_activation:status', (event) => {
          get().setGameActivationStatus(event.payload);
        }),
      ])
        .then(() => undefined)
        .catch((error) => {
          runtimeStatusListenersReady = null;
          throw error;
        });
    }
    await runtimeStatusListenersReady.catch((error) => {
      console.error('Failed to register runtime status listeners', error);
    });
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
      const activation = activeGameId ? await commands.setActiveGame(activeGameId) : null;
      const activeReport = activeGameId ? (startupReportsByGame.get(activeGameId) ?? null) : null;
      const activeReportRevision = activeReport?.reconcile_revision ?? 0;

      if (activation?.game_id) {
        get().setGameActivationStatus({
          game_id: activation.game_id,
          generation: activation.generation,
          phase: activation.phase,
          reconcile_revision: null,
          runtime_sync_generation: null,
          error: null,
        });
      }
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
    const requestSequence = ++activeGameRequestSequence;
    try {
      // The backend resets the per-game disk recovery gate here. Publish the
      // new active ID only afterwards so no workspace query can race ahead and
      // hydrate from a projection created before external/offline changes.
      const activation = await commands.setActiveGame(id);
      if (requestSequence !== activeGameRequestSequence) {
        return;
      }
      const settings = await commands.getSettings();
      if (requestSequence !== activeGameRequestSequence) {
        return;
      }
      queryClient.setQueryData(settingsKeys.all, settings);
      if (activation?.game_id) {
        get().setGameActivationStatus({
          game_id: activation.game_id,
          generation: activation.generation,
          phase: activation.phase,
          reconcile_revision: null,
          runtime_sync_generation: null,
          error: null,
        });
      }
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
      if (requestSequence !== activeGameRequestSequence) {
        return;
      }
      console.error('Failed to sync active game to backend', e);
      toast.error(formatAppError(e));
      throw e;
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
  setRuntimeSyncStatus: (status) =>
    set((state) => {
      const current = state.runtimeSyncByGame[status.game_id];
      if (
        current &&
        (current.generation > status.generation ||
          (current.generation === status.generation &&
            runtimePhaseOrder[current.phase] > runtimePhaseOrder[status.phase]))
      ) {
        return {};
      }
      return {
        runtimeSyncByGame: {
          ...state.runtimeSyncByGame,
          [status.game_id]: status,
        },
      };
    }),
  setGameActivationStatus: (status) =>
    set((state) => {
      if (!status.game_id) {
        return {};
      }
      const current = state.gameActivationByGame[status.game_id];
      const currentPhaseOrder = current ? activationPhaseOrder[current.phase] : -1;
      const nextPhaseOrder = activationPhaseOrder[status.phase];
      if (
        current &&
        (current.generation > status.generation ||
          (current.generation === status.generation &&
            (currentPhaseOrder > nextPhaseOrder ||
              (currentPhaseOrder === nextPhaseOrder &&
                activationStatusInformationScore(current) >=
                  activationStatusInformationScore(status)))))
      ) {
        return {};
      }
      return {
        gameActivationByGame: {
          ...state.gameActivationByGame,
          [status.game_id]: status,
        },
      };
    }),
});
