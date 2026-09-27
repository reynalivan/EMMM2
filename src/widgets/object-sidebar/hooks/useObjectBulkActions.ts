/**
 * useObjectBulkActions — multi-selection operations for ObjectList.
 *
 * Every handler takes the selected id set, walks it, then reports one summary
 * toast. Name formatting and tag parsing live in utils/bulkSummary.
 */

import { formatAppError } from '../../../shared/lib/appError';
import { useState, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { commands, sparse } from '../../../shared/api/tauri/bindings';
import { toast } from '@/shared/ui/toast';
import { useActiveGame } from '@/entities/game';
import { useTranslation } from 'react-i18next';
import { publishRuntimeDescriptor } from '@/shared/lib/queryRefresh';
import {
  admitWorkspaceIntentOverride,
  buildRuntimeMutationDescriptor,
  applyWorkspaceSwitchEffects,
  clearObjectBulkPendingDesired,
  executeWorkspaceObjectBulkSwitch,
  nextWorkspaceIntentRevision,
  runObjectBatchMutation,
  setObjectBulkPendingDesired,
  type RuntimeMutationClass,
  useDeleteObject,
} from '@/features/workspace-runtime';
import type { WorkspaceObjectNode } from '@/entities/workspace';
import { runBulkClassifyAndMatch } from '../utils/runBulkClassifyAndMatch';
import { parseTagList, resolveObjectNames } from '../utils/bulkSummary';
import { truncateNameList } from '../../../shared/lib/hooks/bulkToastMessages';
import { useAppStore } from '@/app/store';

interface BulkDeps {
  objects: WorkspaceObjectNode[];
  setIsSyncing?: unknown;
}

type BulkOutcome = { success: number; failed: number };

interface GameSwitchDrain {
  pendingTargets: Map<string, { enable: boolean; revision: number }>;
  promise: Promise<void> | null;
}

/**
 * Walk `ids` and run `op` on each, counting outcomes instead of aborting.
 *
 * Per-item failures are logged and tallied rather than swallowed, so callers can
 * report what actually happened. Handlers here used to discard the error and
 * then unconditionally claim success.
 */
async function runPerId(ids: Set<string>, op: (id: string) => Promise<void>): Promise<BulkOutcome> {
  let success = 0;
  let failed = 0;

  for (const id of ids) {
    try {
      await op(id);
      success += 1;
    } catch (error) {
      console.error('Bulk operation failed for', id, error);
      failed += 1;
    }
  }

  return { success, failed };
}

export function useObjectBulkActions({ objects }: BulkDeps) {
  const { t } = useTranslation(['objects', 'common']);
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();
  // The batch owns the single trailing refresh; per-item mutation callbacks
  // must not refetch the list after every successful delete.
  const deleteObjectMutation = useDeleteObject({ publishOnSuccess: false });
  const switchDrains = useRef(new Map<string, GameSwitchDrain>());
  const [pendingSwitchGames, setPendingSwitchGames] = useState<Set<string>>(() => new Set());
  const isBulkSwitchPending = activeGame ? pendingSwitchGames.has(activeGame.id) : false;

  const [bulkTagModal, setBulkTagModal] = useState<{
    open: boolean;
    mode: 'add' | 'remove';
  }>({ open: false, mode: 'add' });

  const summarizeSelection = useCallback(
    (ids: Set<string>) =>
      truncateNameList(resolveObjectNames(ids, objects), (extra) =>
        t('objects:bulk.more_others', { count: extra }),
      ),
    [objects, t],
  );

  /** Success toast only when nothing failed; otherwise name the counts. */
  const reportOutcome = useCallback(
    (outcome: BulkOutcome, successMessage: string, verb: string) => {
      if (outcome.failed === 0) {
        toast.success(successMessage);
        return;
      }

      toast.error(
        t('objects:edit_modal.error_message', {
          error: `${verb} ${outcome.success}, failed ${outcome.failed}`,
        }),
      );
    },
    [t],
  );

  const refreshObjectRows = useCallback(
    () =>
      publishRuntimeDescriptor(queryClient, buildRuntimeMutationDescriptor('objectRows'), 'active'),
    [queryClient],
  );

  const handleBulkDelete = useCallback(
    async (ids: Set<string>) => {
      const outcome = await runPerId(ids, async (id) => {
        if (!activeGame) throw new Error('No active game');
        await deleteObjectMutation.mutateAsync({ id, force: false });
      });
      await refreshObjectRows();

      reportOutcome(
        outcome,
        t('objects:delete_dialog.success_bulk', { name: summarizeSelection(ids) }),
        'Deleted',
      );
    },
    [activeGame, deleteObjectMutation, refreshObjectRows, reportOutcome, summarizeSelection, t],
  );

  const handleBulkPin = useCallback(
    async (ids: Set<string>, pin: boolean) => {
      let outcome: BulkOutcome = { success: 0, failed: 0 };

      await runObjectBatchMutation({
        queryClient,
        // Tally per id rather than throwing: the trailing refresh must still run
        // so the pins that did land become visible; a partial failure only needs
        // to be reported.
        mutation: async () => {
          outcome = await runPerId(ids, (id) => commands.pinObject(id, pin));
        },
      });

      const action = pin ? t('objects:bulk.pinned') : t('objects:bulk.unpinned');
      reportOutcome(outcome, `${action} ${summarizeSelection(ids)}`, pin ? 'Pinned' : 'Unpinned');
    },
    [queryClient, reportOutcome, summarizeSelection, t],
  );

  const runBulkSwitch = useCallback(
    (ids: Set<string>, enable: boolean): Promise<void> => {
      if (!activeGame) {
        return Promise.resolve();
      }

      const objectIds = objects
        .filter((candidate) => ids.has(candidate.id))
        .map((object) => object.id);
      if (objectIds.length === 0) {
        return Promise.resolve();
      }
      const gameId = activeGame.id;
      const gameDrain = switchDrains.current.get(gameId) ?? {
        pendingTargets: new Map<string, { enable: boolean; revision: number }>(),
        promise: null,
      };
      switchDrains.current.set(gameId, gameDrain);
      const revision = nextWorkspaceIntentRevision();
      setObjectBulkPendingDesired(gameId, objectIds, enable, revision);
      for (const id of objectIds) {
        gameDrain.pendingTargets.set(id, { enable, revision });
      }
      if (gameDrain.promise) {
        admitWorkspaceIntentOverride(
          gameId,
          objectIds.map((id) => ({ kind: 'object_id', value: id })),
          revision,
        );
        return gameDrain.promise;
      }

      setPendingSwitchGames((current) => new Set(current).add(gameId));
      const drain = async () => {
        try {
          while (gameDrain.pendingTargets.size > 0) {
            const next = gameDrain.pendingTargets.values().next().value;
            if (!next) {
              break;
            }
            const batchIds: string[] = [];
            for (const [id, desired] of gameDrain.pendingTargets) {
              if (desired.enable === next.enable && desired.revision === next.revision) {
                batchIds.push(id);
                gameDrain.pendingTargets.delete(id);
              }
            }
            let result: Awaited<ReturnType<typeof executeWorkspaceObjectBulkSwitch>>;
            try {
              result = await executeWorkspaceObjectBulkSwitch(
                gameId,
                batchIds,
                next.enable,
                next.revision,
              );
            } catch (error) {
              clearObjectBulkPendingDesired(gameId, batchIds, next.revision);
              throw error;
            }
            if (!result || useAppStore.getState().activeGameId !== gameId) {
              clearObjectBulkPendingDesired(gameId, batchIds, next.revision);
              continue;
            }

            const changedCount = result.changed_object_ids.length;
            if (changedCount === 0) {
              clearObjectBulkPendingDesired(gameId, batchIds, next.revision);
              continue;
            }
            const settled = applyWorkspaceSwitchEffects(queryClient, result, 'objectSwitch', {
              gameId,
            });
            const clearDesired = () =>
              clearObjectBulkPendingDesired(gameId, batchIds, next.revision);
            void settled.then(clearDesired, () => undefined);
            const superseded = batchIds.some((id) => gameDrain.pendingTargets.has(id));
            if (superseded) {
              continue;
            }
            const single = changedCount === 1;
            toast.success(
              t(
                next.enable
                  ? single
                    ? 'objects:toasts.enabled_one'
                    : 'objects:toasts.enabled_other'
                  : single
                    ? 'objects:toasts.disabled_one'
                    : 'objects:toasts.disabled_other',
                { count: changedCount },
              ),
            );
          }
        } finally {
          for (const [id, pending] of gameDrain.pendingTargets) {
            clearObjectBulkPendingDesired(gameId, [id], pending.revision);
          }
          gameDrain.pendingTargets.clear();
          gameDrain.promise = null;
          switchDrains.current.delete(gameId);
          setPendingSwitchGames((current) => {
            const next = new Set(current);
            next.delete(gameId);
            return next;
          });
        }
      };
      const promise = drain();
      gameDrain.promise = promise;
      return promise;
    },
    [activeGame, objects, queryClient, t],
  );

  const handleBulkEnable = useCallback(
    (ids: Set<string>) => runBulkSwitch(ids, true),
    [runBulkSwitch],
  );

  const handleBulkDisable = useCallback(
    (ids: Set<string>) => runBulkSwitch(ids, false),
    [runBulkSwitch],
  );

  const applyBulkTags = useCallback(
    async (ids: Set<string>, transform: (existing: string[]) => string[]) => {
      const outcome = await runPerId(ids, async (id) => {
        const obj = objects.find((o) => o.id === id);
        if (!obj) throw new Error(`Object ${id} is no longer in the list`);

        await commands.updateObjectCmd(id, sparse({ tags: transform(parseTagList(obj.tags)) }));
      });
      await refreshObjectRows();

      return outcome;
    },
    [objects, refreshObjectRows],
  );

  const handleBulkAddTags = useCallback(
    async (ids: Set<string>, tagsToAdd: string[]) => {
      const outcome = await applyBulkTags(ids, (existing) => [
        ...new Set([...existing, ...tagsToAdd]),
      ]);

      reportOutcome(
        outcome,
        t('objects:toasts.tags_added', {
          count: tagsToAdd.length,
          items: summarizeSelection(ids),
        }),
        'Tagged',
      );
    },
    [applyBulkTags, reportOutcome, summarizeSelection, t],
  );

  const handleBulkRemoveTags = useCallback(
    async (ids: Set<string>, tagsToRemove: string[]) => {
      const removeSet = new Set(tagsToRemove);
      const outcome = await applyBulkTags(ids, (existing) =>
        existing.filter((tag) => !removeSet.has(tag)),
      );

      reportOutcome(
        outcome,
        t('objects:toasts.tags_removed', {
          count: tagsToRemove.length,
          items: summarizeSelection(ids),
        }),
        'Untagged',
      );
    },
    [applyBulkTags, reportOutcome, summarizeSelection, t],
  );

  const handleBulkClassifyAndMatch = useCallback(
    async (ids: Set<string>) => {
      await runBulkClassifyAndMatch({
        ids,
        activeGame,
        t,
      });
    },
    [activeGame, t],
  );

  /** Run a bulk folder-path command, refresh the rows, and report the outcome. */
  const runBulkFolderCommand = useCallback(
    async (
      ids: Set<string>,
      run: (gameId: string, paths: string[]) => Promise<unknown>,
      successKey: string,
      mutationClass: RuntimeMutationClass = 'objectRows',
    ) => {
      if (!activeGame) return;
      const paths = objects.filter((o) => ids.has(o.id)).map((o) => o.folder_path);
      try {
        await run(activeGame.id, paths);
        await publishRuntimeDescriptor(
          queryClient,
          buildRuntimeMutationDescriptor(mutationClass),
          'active',
        );
        toast.success(t(successKey, { count: ids.size }));
      } catch (e) {
        toast.error(t('objects:edit_modal.error_message', { error: formatAppError(e) }));
      }
    },
    [activeGame, objects, queryClient, t],
  );

  const handleBulkFavorite = useCallback(
    (ids: Set<string>, favorite: boolean) =>
      runBulkFolderCommand(
        ids,
        (gameId, paths) => commands.bulkToggleFavorite(gameId, paths, favorite),
        favorite ? 'objects:toasts.favorite_added_other' : 'objects:toasts.favorite_removed_other',
      ),
    [runBulkFolderCommand],
  );

  const handleBulkSafe = useCallback(
    (ids: Set<string>, safe: boolean) =>
      runBulkFolderCommand(
        ids,
        (gameId, paths) => commands.bulkSetModSafety(gameId, paths, safe),
        safe ? 'objects:toasts.mark_safe' : 'objects:toasts.mark_unsafe',
        'safetyClassification',
      ),
    [runBulkFolderCommand],
  );

  return {
    bulkTagModal,
    setBulkTagModal,
    isBulkSwitchPending,
    handleBulkDelete,
    handleBulkPin,
    handleBulkEnable,
    handleBulkDisable,
    handleBulkAddTags,
    handleBulkRemoveTags,
    handleBulkClassifyAndMatch,
    handleBulkFavorite,
    handleBulkSafe,
  };
}
