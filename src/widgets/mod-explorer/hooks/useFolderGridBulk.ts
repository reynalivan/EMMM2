import { useCallback, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useActiveGame } from '@/entities/game';
import { thumbnailKeys, type MoveStatus } from '@/entities/mod';
import type { ModFolder } from '@/entities/game-object';
import type { WorkspaceExplorerQuery, WorkspaceExplorerSelectionModel } from '@/entities/workspace';
import {
  WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT,
  toWorkspaceExplorerSelectionInput,
  workspaceExplorerSelectionCount,
} from '@/entities/workspace';
import type {
  BulkResult,
  WorkspaceExplorerBulkAction,
  WorkspaceExplorerSelectionInput,
} from '@/shared/api/tauri/bindings.gen';
import { commands, sparse } from '@/shared/api/tauri/bindings';
import { formatAppError, isExplorerSnapshotExpired } from '@/shared/lib/appError';
import {
  formatBulkCancelledMessage,
  formatBulkFailureMessage,
  formatBulkSuccessMessage,
} from '@/shared/lib/hooks/bulkToastMessages';
import { toast } from '@/shared/ui/toast';
import { useAppStore } from '@/app/store';
import {
  admitWorkspaceIntentOverride,
  applyRuntimeEffects,
  applyRuntimeMutationResult,
  buildQueryRemovalDescriptor,
  buildWorkspacePathRewritesDescriptor,
  clearFolderBulkPendingDesired,
  ensureWorkspaceProjectionListener,
  normalizeWorkspacePath,
  nextWorkspaceIntentRevision,
  parseRenameConflict,
  publishCollectionReferenceImpact,
  setFolderBulkPendingDesired,
  showWorkspaceRenameConflictDialog,
  waitForWorkspaceProjection,
  workspaceKeys,
} from '@/features/workspace-runtime';
import { notifyCommittedMutationSyncWarning } from '@/shared/lib/committedMutationWarning';
import { useTranslation } from 'react-i18next';

interface FolderGridBulkOptions {
  selection: WorkspaceExplorerSelectionModel;
  explorerQuery: WorkspaceExplorerQuery | null;
  listingRevision: string | null;
  selectionStable?: boolean;
  sortedFolders: ModFolder[];
  clearGridSelection: () => void;
  removeGridSelectionPaths: (paths: Iterable<string>) => void;
  openMoveDialog: (folder: ModFolder) => void;
}

interface BulkExecutionSnapshot {
  gameId: string;
  selection: WorkspaceExplorerSelectionInput;
  intentRevision?: number;
}

type ToggleAction = Extract<WorkspaceExplorerBulkAction, { kind: 'toggle' }>;

interface ToggleExecutionSnapshot {
  action: ToggleAction;
  execution: BulkExecutionSnapshot;
  overlayPaths: string[];
  selectionIdentity: WorkspaceExplorerSelectionModel;
  listingRevision: string;
}

function sameToggleSelection(
  left: ToggleExecutionSnapshot,
  right: ToggleExecutionSnapshot,
): boolean {
  if (
    left.execution.gameId !== right.execution.gameId ||
    left.listingRevision !== right.listingRevision
  ) {
    return false;
  }
  if (left.selectionIdentity === right.selectionIdentity) {
    return true;
  }
  const a = left.execution.selection;
  const b = right.execution.selection;
  if (
    JSON.stringify(a.query) !== JSON.stringify(b.query) ||
    a.selection.mode !== b.selection.mode
  ) {
    return false;
  }
  const aPaths = a.selection.mode === 'explicit' ? a.selection.paths : a.selection.excluded_paths;
  const bPaths = b.selection.mode === 'explicit' ? b.selection.paths : b.selection.excluded_paths;
  if (aPaths.length !== bPaths.length) {
    return false;
  }
  const bSet = new Set(bPaths);
  return aPaths.every((path) => bSet.has(path));
}

interface BulkMoveSnapshot extends BulkExecutionSnapshot {
  paths: string[];
}

function createBulkOperationId(kind: 'toggle' | 'delete'): string {
  return `${kind}-${crypto.randomUUID()}`;
}

function isActiveGame(gameId: string): boolean {
  return useAppStore.getState().activeGameId === gameId;
}

function normalizedPathKey(path: string): string {
  return normalizeWorkspacePath(path).toLocaleLowerCase('en-US');
}

function committedMoveSourcePaths(sourcePaths: string[], result: BulkResult): string[] {
  if (
    result.cancelled ||
    result.unprocessed_count > 0 ||
    result.processed_count !== sourcePaths.length ||
    result.success.length + result.failures.length !== sourcePaths.length
  ) {
    return [];
  }

  const failedPaths = new Set(result.failures.map((failure) => normalizedPathKey(failure.path)));
  const committedPaths = sourcePaths.filter((path) => !failedPaths.has(normalizedPathKey(path)));
  return committedPaths.length === result.success.length ? committedPaths : [];
}

function showBulkResult(action: WorkspaceExplorerBulkAction, result: BulkResult): void {
  if (action.kind === 'move_to_object') {
    return;
  }

  if (result.success.length > 0) {
    const successAction =
      action.kind === 'toggle'
        ? action.enable
          ? 'enabled'
          : 'disabled'
        : action.kind === 'delete'
          ? 'deleted'
          : action.kind === 'set_safety'
            ? action.safe
              ? 'marked_safe'
              : 'marked_unsafe'
            : action.kind === 'set_favorite'
              ? action.favorite
                ? 'favorited'
                : 'unfavorited'
              : action.kind === 'set_pin'
                ? action.pin
                  ? 'pinned'
                  : 'unpinned'
                : 'updated';
    toast.success(formatBulkSuccessMessage(result.success, successAction));
  }
  const reportableFailures = result.failures.filter(
    (failure) => action.kind !== 'toggle' || !parseRenameConflict(failure.error),
  );
  if (reportableFailures.length > 0) {
    const failureAction =
      action.kind === 'set_safety'
        ? 'safety'
        : action.kind === 'set_favorite'
          ? 'favorite'
          : action.kind === 'set_pin'
            ? 'pin'
            : action.kind === 'update_info'
              ? 'update'
              : action.kind;
    toast.error(formatBulkFailureMessage(reportableFailures, failureAction));
  }
  if (result.cancelled) {
    toast.info(formatBulkCancelledMessage(result));
  }
}

function refreshInBackground(task: Promise<void>, label: string): void {
  void task.catch((error: unknown) => {
    console.error(`[FolderGridBulk] ${label} refresh failed:`, error);
  });
}

export function useFolderGridBulk({
  selection,
  explorerQuery,
  listingRevision,
  selectionStable = true,
  sortedFolders,
  clearGridSelection,
  removeGridSelectionPaths,
  openMoveDialog,
}: FolderGridBulkOptions) {
  const { t } = useTranslation(['grid']);
  const { activeGame } = useActiveGame();
  const activeGameId = activeGame?.id;
  const queryClient = useQueryClient();
  const [bulkTagOpen, setBulkTagOpen] = useState(false);
  const [bulkDeleteConfirm, setBulkDeleteConfirm] = useState(false);
  const [bulkMutationPending, setBulkMutationPending] = useState(false);
  const [bulkMoveSnapshot, setBulkMoveSnapshot] = useState<BulkMoveSnapshot | null>(null);
  const bulkMutationInFlight = useRef(false);
  const runningToggle = useRef<ToggleExecutionSnapshot | null>(null);
  const pendingToggle = useRef<ToggleExecutionSnapshot | null>(null);

  const beginBulkMutation = useCallback(() => {
    if (bulkMutationInFlight.current) {
      return false;
    }
    bulkMutationInFlight.current = true;
    setBulkMutationPending(true);
    return true;
  }, []);
  const finishBulkMutation = useCallback(() => {
    bulkMutationInFlight.current = false;
    setBulkMutationPending(false);
  }, []);

  const executeBulkAction = useCallback(
    async (
      action: WorkspaceExplorerBulkAction,
      frozenSnapshot?: BulkExecutionSnapshot,
      togglePaths?: string[],
      onToggleSettled?: () => void,
    ): Promise<BulkResult> => {
      let snapshot = frozenSnapshot;
      if (!snapshot) {
        if (!selectionStable || !activeGameId || !explorerQuery || !listingRevision) {
          throw new Error('No active explorer snapshot');
        }
        snapshot = {
          gameId: activeGameId,
          selection: toWorkspaceExplorerSelectionInput(selection, explorerQuery, listingRevision),
        };
      }
      const execute = () =>
        action.kind === 'toggle' && togglePaths
          ? commands.bulkToggleMods(
              snapshot.gameId,
              togglePaths,
              action.enable,
              action.operation_id,
              snapshot.intentRevision ?? null,
            )
          : commands.executeWorkspaceExplorerBulk(
              {
                selection: snapshot.selection,
                action,
              },
              snapshot.intentRevision ?? null,
            );
      if (action.kind === 'toggle') {
        await ensureWorkspaceProjectionListener();
      }
      const result = await execute();
      if (!isActiveGame(snapshot.gameId)) {
        onToggleSettled?.();
        return result;
      }

      if (result.path_rewrites.length > 0) {
        applyRuntimeEffects(
          queryClient,
          buildWorkspacePathRewritesDescriptor(result.path_rewrites, []),
        );
      }
      if (action.kind === 'delete' || action.kind === 'move_to_object') {
        const removedPaths =
          action.kind === 'move_to_object'
            ? result.path_rewrites.map((rewrite) => rewrite.old_path)
            : result.success;
        applyRuntimeEffects(
          queryClient,
          buildQueryRemovalDescriptor(
            removedPaths.map((path) => thumbnailKeys.folder(path)),
            [],
          ),
        );
      }

      if (action.kind === 'toggle') {
        const refresh = (
          result.disk_revision === null
            ? Promise.resolve()
            : waitForWorkspaceProjection(snapshot.gameId, result.disk_revision)
        ).then(() => applyRuntimeMutationResult(queryClient, 'folderSwitch'));
        refreshInBackground(refresh.then(onToggleSettled), 'workspace');
      } else if (action.kind === 'delete') {
        refreshInBackground(
          applyRuntimeMutationResult(queryClient, [
            'workspaceStructure',
            'workspaceRuntime',
            'dashboardKeybindings',
          ]),
          'workspace',
        );
      } else if (action.kind === 'set_safety') {
        refreshInBackground(
          applyRuntimeMutationResult(queryClient, 'safetyClassification'),
          'workspace',
        );
      } else if (action.kind === 'move_to_object') {
        refreshInBackground(
          applyRuntimeMutationResult(queryClient, 'workspaceStructure'),
          'workspace',
        );
      } else {
        refreshInBackground(
          applyRuntimeMutationResult(queryClient, 'folderMetadataPreview'),
          'workspace',
        );
      }
      refreshInBackground(
        publishCollectionReferenceImpact(queryClient, result.collection_impact),
        'collections',
      );
      notifyCommittedMutationSyncWarning(result);
      if (action.kind === 'toggle') {
        const folderCollision = result.failures.find((failure) =>
          parseRenameConflict(failure.error),
        );
        if (folderCollision) {
          void showWorkspaceRenameConflictDialog(snapshot.gameId, folderCollision.error);
        }
      }
      if (action.kind !== 'toggle' || pendingToggle.current === null) {
        showBulkResult(action, result);
      }
      return result;
    },
    [activeGameId, explorerQuery, listingRevision, queryClient, selection, selectionStable],
  );

  const runBulkAction = useCallback(
    (action: WorkspaceExplorerBulkAction, onSuccess?: (result: BulkResult) => void) => {
      if (!selectionStable) {
        return;
      }
      const selectionCount = workspaceExplorerSelectionCount(selection);
      if (selectionCount === 0) {
        return;
      }
      if (selectionCount > WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT) {
        toast.error(
          t('bulk.selection_limit', {
            limit: WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT.toLocaleString('en-US'),
          }),
        );
        return;
      }
      let toggleSnapshot: ToggleExecutionSnapshot | null = null;
      if (action.kind === 'toggle') {
        if (!activeGameId || !explorerQuery || !listingRevision) {
          toast.error(t('common:errors.explorer_snapshot_expired'));
          return;
        }
        const intentRevision = nextWorkspaceIntentRevision();
        toggleSnapshot = {
          action,
          execution: {
            gameId: activeGameId,
            selection: toWorkspaceExplorerSelectionInput(selection, explorerQuery, listingRevision),
            intentRevision,
          },
          overlayPaths:
            selection.mode === 'explicit'
              ? [...selection.paths]
              : sortedFolders
                  .map((folder) => folder.path)
                  .filter((path) => !selection.excludedPaths.has(path)),
          selectionIdentity: selection,
          listingRevision,
        };
        if (bulkMutationInFlight.current) {
          const running = runningToggle.current;
          if (!running || !sameToggleSelection(running, toggleSnapshot)) {
            toast.error(t('common:errors.explorer_snapshot_expired'));
            return;
          }
          setFolderBulkPendingDesired(
            activeGameId,
            toggleSnapshot.overlayPaths,
            action.enable,
            intentRevision,
          );
          admitWorkspaceIntentOverride(
            activeGameId,
            toggleSnapshot.overlayPaths.map((path) => ({ kind: 'mod_path', value: path })),
            intentRevision,
          );
          pendingToggle.current = toggleSnapshot;
          return;
        }
      }
      if (!beginBulkMutation()) {
        return;
      }
      if (toggleSnapshot) {
        setFolderBulkPendingDesired(
          toggleSnapshot.execution.gameId,
          toggleSnapshot.overlayPaths,
          toggleSnapshot.action.enable,
          toggleSnapshot.execution.intentRevision ?? 0,
        );
      }
      const handleError = async (error: unknown) => {
        if (isExplorerSnapshotExpired(error) && explorerQuery) {
          clearGridSelection();
          await queryClient.resetQueries({
            queryKey: workspaceKeys.explorerPages(explorerQuery),
            exact: true,
          });
        }
        toast.error(formatAppError(error));
      };
      if (toggleSnapshot) {
        const runToggle = async () => {
          let next: ToggleExecutionSnapshot | null = toggleSnapshot;
          let committedPaths: string[] | null = null;
          while (next) {
            const current = next;
            runningToggle.current = current;
            try {
              const result = await executeBulkAction(
                current.action,
                current.execution,
                committedPaths ?? undefined,
                () =>
                  clearFolderBulkPendingDesired(
                    current.execution.gameId,
                    current.overlayPaths,
                    current.execution.intentRevision ?? 0,
                  ),
              );
              const originalPaths =
                current.execution.selection.selection.mode === 'explicit'
                  ? current.execution.selection.selection.paths
                  : [];
              const renamedPaths = new Map(
                result.path_rewrites.map((rewrite) => [rewrite.old_path, rewrite.new_path]),
              );
              const continuationPaths = [
                ...originalPaths.map((path) => renamedPaths.get(path) ?? path),
                ...result.success,
                ...result.failures.map((failure) => failure.path),
              ];
              committedPaths =
                continuationPaths.length > 0 ? [...new Set(continuationPaths)] : null;
            } catch (error) {
              clearFolderBulkPendingDesired(
                current.execution.gameId,
                current.overlayPaths,
                current.execution.intentRevision ?? 0,
              );
              await handleError(error);
              committedPaths = null;
            }
            next = pendingToggle.current;
            pendingToggle.current = null;
            if (next && committedPaths === null) {
              clearFolderBulkPendingDesired(
                next.execution.gameId,
                next.overlayPaths,
                next.execution.intentRevision ?? 0,
              );
              toast.error(t('common:errors.explorer_snapshot_expired'));
              break;
            }
          }
        };
        void runToggle().finally(() => {
          runningToggle.current = null;
          finishBulkMutation();
        });
        return;
      }
      void executeBulkAction(action).then(onSuccess).catch(handleError).finally(finishBulkMutation);
    },
    [
      activeGameId,
      beginBulkMutation,
      clearGridSelection,
      executeBulkAction,
      explorerQuery,
      finishBulkMutation,
      listingRevision,
      queryClient,
      selection,
      selectionStable,
      sortedFolders,
      t,
    ],
  );

  const handleBulkToggle = useCallback(
    (enable: boolean) => {
      runBulkAction({ kind: 'toggle', enable, operation_id: createBulkOperationId('toggle') });
    },
    [runBulkAction],
  );

  const handleBulkTagRequest = useCallback(() => {
    if (!selectionStable) {
      return;
    }
    setBulkTagOpen(true);
  }, [selectionStable]);

  const handleBulkTagSubmit = useCallback(
    (tags: string[]) => {
      runBulkAction({ kind: 'update_info', update: sparse({ tags_add: tags }) });
    },
    [runBulkAction],
  );

  const handleBulkDeleteRequest = useCallback(() => {
    if (!selectionStable) {
      return;
    }
    setBulkDeleteConfirm(true);
  }, [selectionStable]);

  const handleBulkDeleteConfirm = useCallback(() => {
    runBulkAction({ kind: 'delete', operation_id: createBulkOperationId('delete') }, () => {
      setBulkDeleteConfirm(false);
      clearGridSelection();
    });
  }, [clearGridSelection, runBulkAction]);

  const handleBulkFavorite = useCallback(
    (favorite: boolean) => {
      runBulkAction({ kind: 'set_favorite', favorite });
    },
    [runBulkAction],
  );

  const handleBulkSafe = useCallback(
    (safe: boolean) => {
      runBulkAction({ kind: 'set_safety', safe });
    },
    [runBulkAction],
  );

  const handleBulkPin = useCallback(
    (pin: boolean) => {
      runBulkAction({ kind: 'set_pin', pin });
    },
    [runBulkAction],
  );

  const handleBulkMoveToObject = useCallback(() => {
    if (!selectionStable || !activeGameId || !explorerQuery || !listingRevision) {
      return;
    }
    const selectionCount = workspaceExplorerSelectionCount(selection);
    if (selectionCount > WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT) {
      toast.error(
        t('bulk.selection_limit', {
          limit: WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT.toLocaleString('en-US'),
        }),
      );
      return;
    }
    if (selection.mode !== 'explicit' || selection.paths.size === 0) {
      toast.error(t('bulk.move_requires_loaded_selection'));
      return;
    }

    const foldersByPath = new Map(sortedFolders.map((folder) => [folder.path, folder]));
    const exactPaths = Array.from(selection.paths);
    const firstSelected = foldersByPath.get(exactPaths[0]);
    if (!firstSelected || exactPaths.some((path) => !foldersByPath.has(path))) {
      toast.error(t('bulk.move_requires_loaded_selection'));
      return;
    }

    const selectionInput = toWorkspaceExplorerSelectionInput(
      selection,
      explorerQuery,
      listingRevision,
    );
    setBulkMoveSnapshot({
      gameId: activeGameId,
      paths: exactPaths,
      selection: {
        ...selectionInput,
        query: { ...selectionInput.query },
        selection: { mode: 'explicit', paths: [...exactPaths] },
      },
    });
    openMoveDialog(firstSelected);
  }, [
    activeGameId,
    explorerQuery,
    listingRevision,
    openMoveDialog,
    selection,
    selectionStable,
    sortedFolders,
    t,
  ]);

  const handleBulkMoveSubmit = useCallback(
    async (targetObjectId: string, status: MoveStatus, targetSubpath: string | null) => {
      if (!bulkMoveSnapshot) {
        throw new Error('The exact bulk move selection is unavailable');
      }
      if (!isActiveGame(bulkMoveSnapshot.gameId)) {
        throw new Error(t('bulk.move_requires_loaded_selection'));
      }
      if (!beginBulkMutation()) {
        throw new Error('A bulk operation is already running');
      }
      try {
        const result = await executeBulkAction(
          {
            kind: 'move_to_object',
            target_object_id: targetObjectId,
            target_subpath: targetSubpath,
            status,
          },
          bulkMoveSnapshot,
        );
        const committedPaths = committedMoveSourcePaths(bulkMoveSnapshot.paths, result);
        if (committedPaths.length > 0) {
          removeGridSelectionPaths(committedPaths);
        }
        if (result.success.length > 0) {
          toast.success(t('bulk.move_success', { count: result.success.length }));
        }
        if (result.failures.length > 0) {
          toast.error(
            t('bulk.move_failure', {
              count: result.failures.length,
              error: formatAppError(result.failures[0].error),
            }),
          );
        }
      } finally {
        finishBulkMutation();
      }
    },
    [
      beginBulkMutation,
      bulkMoveSnapshot,
      executeBulkAction,
      finishBulkMutation,
      removeGridSelectionPaths,
      t,
    ],
  );

  return {
    bulkMutationPending,
    bulkTagOpen,
    setBulkTagOpen,
    bulkDeleteConfirm,
    setBulkDeleteConfirm,
    bulkMovePaths: bulkMoveSnapshot?.paths ?? null,
    clearBulkMovePaths: () => setBulkMoveSnapshot(null),
    handleBulkToggle,
    handleBulkTagRequest,
    handleBulkTagSubmit,
    handleBulkDeleteRequest,
    handleBulkDeleteConfirm,
    handleBulkFavorite,
    handleBulkSafe,
    handleBulkPin,
    handleBulkMoveToObject,
    handleBulkMoveSubmit,
  };
}
