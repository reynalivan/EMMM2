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
  applyRuntimeEffects,
  applyRuntimeMutationResult,
  buildQueryRemovalDescriptor,
  buildWorkspacePathRewritesDescriptor,
  enqueueWorkspaceGameMutation,
  normalizeWorkspacePath,
  publishCollectionReferenceImpact,
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
  if (result.failures.length > 0) {
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
    toast.error(formatBulkFailureMessage(result.failures, failureAction));
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
        commands.executeWorkspaceExplorerBulk({
          selection: snapshot.selection,
          action,
        });
      const result =
        action.kind === 'toggle'
          ? await enqueueWorkspaceGameMutation(snapshot.gameId, execute)
          : await execute();
      if (!isActiveGame(snapshot.gameId)) {
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
        refreshInBackground(applyRuntimeMutationResult(queryClient, 'folderSwitch'), 'workspace');
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
      showBulkResult(action, result);
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
      if (!beginBulkMutation()) {
        return;
      }
      void executeBulkAction(action)
        .then(onSuccess)
        .catch(async (error: unknown) => {
          if (isExplorerSnapshotExpired(error) && explorerQuery) {
            clearGridSelection();
            await queryClient.resetQueries({
              queryKey: workspaceKeys.explorerPages(explorerQuery),
              exact: true,
            });
          }
          toast.error(formatAppError(error));
        })
        .finally(finishBulkMutation);
    },
    [
      beginBulkMutation,
      clearGridSelection,
      executeBulkAction,
      explorerQuery,
      finishBulkMutation,
      queryClient,
      selection,
      selectionStable,
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
