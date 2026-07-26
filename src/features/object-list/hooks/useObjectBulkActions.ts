/**
 * useObjectBulkActions — multi-selection operations for ObjectList.
 *
 * Every handler takes the selected id set, walks it, then reports one summary
 * toast. Name formatting and tag parsing live in utils/bulkSummary.
 */

import { useState, useCallback, type Dispatch, type SetStateAction } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { commands, sparse } from '../../../lib/bindings';
import { toast } from '../../../stores/useToastStore';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { runObjectBatchMutation } from '../../../hooks/objectQueryCache';
import { useDeleteObject } from '../../../hooks/useObjectMutations';
import { useTranslation } from 'react-i18next';
import { publishRuntimeDescriptor } from '../../runtime-sync/queryRefresh';
import { buildRuntimeMutationDescriptor } from '../../workspace-runtime/optimistic/descriptorBuilders';
import { useWorkspaceSwitchActions } from '../../workspace-runtime/actions/useWorkspaceSwitchActions';
import type { WorkspaceObjectNode } from '../../../types/workspace';
import { runBulkAutoRecognize } from '../utils/runBulkAutoRecognize';
import { parseTagList, resolveObjectNames, truncateNameList } from '../utils/bulkSummary';

interface BulkDeps {
  objects: WorkspaceObjectNode[];
  setIsSyncing: Dispatch<SetStateAction<boolean>>;
}

type BulkOutcome = { success: number; failed: number };

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

export function useObjectBulkActions({ objects, setIsSyncing }: BulkDeps) {
  const { t } = useTranslation(['objects', 'common']);
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();
  const deleteObjectMutation = useDeleteObject();
  const switchActions = useWorkspaceSwitchActions();

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
    async (ids: Set<string>, enable: boolean) => {
      if (!activeGame) {
        return;
      }

      let successCount = 0;
      let failedCount = 0;
      for (const object of objects.filter((candidate) => ids.has(candidate.id))) {
        const nextPath = await switchActions.setNodeEnabled(object, enable, 'object_list', {
          syncExplorerPath: false,
        });
        if (nextPath) {
          successCount += 1;
          continue;
        }
        failedCount += 1;
      }

      if (failedCount === 0) {
        const single = successCount === 1;
        toast.success(
          t(
            enable
              ? single
                ? 'objects:toasts.enabled_one'
                : 'objects:toasts.enabled_other'
              : single
                ? 'objects:toasts.disabled_one'
                : 'objects:toasts.disabled_other',
            { count: successCount },
          ),
        );
        return;
      }

      toast.error(`${enable ? 'Enabled' : 'Disabled'} ${successCount}, failed ${failedCount}`);
    },
    [activeGame, objects, switchActions, t],
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

  const handleBulkAutoRecognize = useCallback(
    async (ids: Set<string>) => {
      await runBulkAutoRecognize({
        ids,
        activeGame,
        objects,
        queryClient,
        setIsSyncing,
        t,
      });
    },
    [activeGame, objects, queryClient, setIsSyncing, t],
  );

  const handleBulkFavorite = useCallback(
    async (ids: Set<string>, favorite: boolean) => {
      if (!activeGame) return;
      const paths = objects.filter((o) => ids.has(o.id)).map((o) => o.folder_path);
      try {
        await commands.bulkToggleFavorite(activeGame.id, paths, favorite);
        await refreshObjectRows();
        toast.success(
          t(
            favorite
              ? 'objects:toasts.favorite_added_other'
              : 'objects:toasts.favorite_removed_other',
            {
              count: ids.size,
            },
          ),
        );
      } catch (e) {
        toast.error(t('objects:edit_modal.error_message', { error: String(e) }));
      }
    },
    [activeGame, objects, refreshObjectRows, t],
  );

  const handleBulkSafe = useCallback(
    async (ids: Set<string>, safe: boolean) => {
      if (!activeGame) return;
      const paths = objects.filter((o) => ids.has(o.id)).map((o) => o.folder_path);
      try {
        await commands.bulkUpdateInfo(activeGame.id, paths, sparse({ is_safe: safe }));
        await refreshObjectRows();
        toast.success(
          t(safe ? 'objects:toasts.mark_safe' : 'objects:toasts.mark_unsafe', {
            count: ids.size,
          }),
        );
      } catch (e) {
        toast.error(t('objects:edit_modal.error_message', { error: String(e) }));
      }
    },
    [activeGame, objects, refreshObjectRows, t],
  );

  return {
    bulkTagModal,
    setBulkTagModal,
    handleBulkDelete,
    handleBulkPin,
    handleBulkEnable,
    handleBulkDisable,
    handleBulkAddTags,
    handleBulkRemoveTags,
    handleBulkAutoRecognize,
    handleBulkFavorite,
    handleBulkSafe,
  };
}
