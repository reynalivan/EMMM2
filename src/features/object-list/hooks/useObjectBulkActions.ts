/**
 * useObjectBulkActions — multi-selection operations for ObjectList.
 *
 * Every handler takes the selected id set, walks it, then reports one summary
 * toast. Name formatting and tag parsing live in utils/bulkSummary.
 */

import { useState, useCallback, type Dispatch, type SetStateAction } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { commands } from '../../../lib/bindings';
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
import {
  othersSuffix,
  parseTagList,
  resolveObjectNames,
  truncateNameList,
} from '../utils/bulkSummary';

interface BulkDeps {
  objects: WorkspaceObjectNode[];
  setIsSyncing: Dispatch<SetStateAction<boolean>>;
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
    (ids: Set<string>) => truncateNameList(resolveObjectNames(ids, objects), othersSuffix),
    [objects],
  );

  const refreshObjectRows = useCallback(
    () =>
      publishRuntimeDescriptor(queryClient, buildRuntimeMutationDescriptor('objectRows'), 'active'),
    [queryClient],
  );

  const handleBulkDelete = useCallback(
    async (ids: Set<string>) => {
      let success = 0;
      let failed = 0;
      for (const id of ids) {
        try {
          if (!activeGame) throw new Error('No active game');
          await deleteObjectMutation.mutateAsync({ id, force: false });
          success++;
        } catch {
          failed++;
        }
      }
      await refreshObjectRows();

      if (failed === 0) {
        toast.success(t('objects:delete_dialog.success_bulk', { name: summarizeSelection(ids) }));
        return;
      }

      toast.error(
        t('objects:edit_modal.error_message', {
          error: `Deleted ${success}, failed ${failed}`,
        }),
      );
    },
    [activeGame, deleteObjectMutation, refreshObjectRows, summarizeSelection, t],
  );

  const handleBulkPin = useCallback(
    async (ids: Set<string>, pin: boolean) => {
      try {
        await runObjectBatchMutation({
          queryClient,
          applyOptimisticUpdate: (object) =>
            ids.has(object.id)
              ? {
                  ...object,
                  is_pinned: pin,
                }
              : object,
          mutation: async () => {
            for (const id of ids) {
              await commands.pinObject({ id, pin });
            }
          },
        });
      } catch (e) {
        console.error('Bulk pin failed', e);
      }

      const action = pin ? t('objects:bulk.pinned') : t('objects:bulk.unpinned');
      const names = truncateNameList(resolveObjectNames(ids, objects), (extra) =>
        t('objects:bulk.more_others', { count: extra }),
      );

      toast.success(`${action} ${names}`);
    },
    [objects, queryClient, t],
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
    async (ids: Set<string>, transform: (existing: string[]) => string[], logLabel: string) => {
      for (const id of ids) {
        const obj = objects.find((o) => o.id === id);
        if (!obj) continue;
        try {
          await commands.updateObject({
            id,
            updates: { tags: transform(parseTagList(obj.tags)) },
          });
        } catch (e) {
          console.error(logLabel, id, e);
        }
      }
      await refreshObjectRows();
    },
    [objects, refreshObjectRows],
  );

  const handleBulkAddTags = useCallback(
    async (ids: Set<string>, tagsToAdd: string[]) => {
      await applyBulkTags(
        ids,
        (existing) => [...new Set([...existing, ...tagsToAdd])],
        'Bulk add tags failed for',
      );

      toast.success(
        t('objects:toasts.tags_added', {
          count: tagsToAdd.length,
          items: summarizeSelection(ids),
        }),
      );
    },
    [applyBulkTags, summarizeSelection, t],
  );

  const handleBulkRemoveTags = useCallback(
    async (ids: Set<string>, tagsToRemove: string[]) => {
      const removeSet = new Set(tagsToRemove);
      await applyBulkTags(
        ids,
        (existing) => existing.filter((tag) => !removeSet.has(tag)),
        'Bulk remove tags failed for',
      );

      toast.success(
        t('objects:toasts.tags_removed', {
          count: tagsToRemove.length,
          items: summarizeSelection(ids),
        }),
      );
    },
    [applyBulkTags, summarizeSelection, t],
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
        await commands.bulkToggleFavorite({
          gameId: activeGame.id,
          folderPaths: paths,
          favorite,
        });
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
        await commands.bulkUpdateInfo({
          gameId: activeGame.id,
          paths,
          update: { is_safe: safe },
        });
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
