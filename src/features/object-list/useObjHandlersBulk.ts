/**
 * useObjHandlersBulk — Bulk operation handlers for ObjectList.
 * Extracted from useObjectListHandlers for SRP.
 */

import { useState, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { commands } from '../../lib/bindings';
import { toast } from '../../stores/useToastStore';
import type { ObjectSummary, UpdateObjectInput } from '../../types/object';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useDeleteCollection as useDeleteObject } from '../collections/hooks/useCollections';
import { scanService } from '../../lib/services/scanService';
import type { GameType } from '../../types/game';
import { useTranslation } from 'react-i18next';

interface BulkDeps {
  objects: ObjectSummary[];
  toggleObjectMods: (objectId: string, enable: boolean, suppressToast?: boolean) => Promise<void>;
}

/**
 * Helper to create an UpdateObjectInput with all fields set to null by default.
 * Defined outside to avoid hook dependency issues.
 */
function createObjectUpdate(patch: Partial<UpdateObjectInput>): UpdateObjectInput {
  return {
    name: null,
    object_type: null,
    sub_category: null,
    status: null,
    metadata: null,
    hash_db: null,
    custom_skins: null,
    thumbnail_path: null,
    is_auto_sync: null,
    tags: null,
    ...patch,
  };
}

export function useObjHandlersBulk({ objects, toggleObjectMods }: BulkDeps) {
  const { t } = useTranslation(['objects', 'common']);
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();
  const deleteObjectMutation = useDeleteObject();

  const [bulkTagModal, setBulkTagModal] = useState<{
    open: boolean;
    mode: 'add' | 'remove';
  }>({ open: false, mode: 'add' });

  const handleBulkDelete = useCallback(
    async (ids: Set<string>) => {
      let success = 0;
      let failed = 0;
      for (const id of ids) {
        try {
          if (!activeGame) throw new Error('No active game');
          await deleteObjectMutation.mutateAsync({ gameId: activeGame.id, id: id });
          success++;
        } catch {
          failed++;
        }
      }
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });

      const count = ids.size;
      const displayNames = Array.from(ids).map((id) => {
        const obj = objects.find((o) => o.id === id);
        return obj ? obj.name : id;
      });

      const itemsStr =
        count <= 4
          ? displayNames.join(', ')
          : `${displayNames.slice(0, 4).join(', ')} + ${count - 4} others`;

      if (failed === 0) {
        toast.success(t('objects:delete_dialog.success_bulk', { name: itemsStr }));
      } else {
        toast.error(
          t('objects:edit_modal.error_message', {
            error: `Deleted ${success}, failed ${failed}`,
          }),
        );
      }
    },
    [deleteObjectMutation, queryClient, objects, activeGame, t],
  );

  const handleBulkPin = useCallback(
    async (ids: Set<string>, pin: boolean) => {
      for (const id of ids) {
        try {
          await commands.pinObject({ id, isPinned: pin });
        } catch (e) {
          console.error('Bulk pin failed for', id, e);
        }
      }
      queryClient.invalidateQueries({ queryKey: ['objects'] });

      const count = ids.size;
      const displayNames = Array.from(ids).map((id) => {
        const obj = objects.find((o) => o.id === id);
        return obj ? obj.name : id;
      });

      const action = pin
        ? t('objects:bulk.pinned')
        : t('objects:bulk.unpinned', { defaultValue: 'Unpinned' });
      const toastMsg =
        count <= 4
          ? `${action} ${displayNames.join(', ')}`
          : `${action} ${displayNames.slice(0, 4).join(', ')} + ${count - 4} others`;

      toast.success(toastMsg);
    },
    [queryClient, objects, t],
  );

  const handleBulkEnable = useCallback(
    async (ids: Set<string>) => {
      for (const id of ids) await toggleObjectMods(id, true, true);

      const toastMsg = t(
        ids.size === 1 ? 'objects:toasts.enabled_one' : 'objects:toasts.enabled_other',
        {
          count: ids.size,
        },
      );

      toast.success(toastMsg);
    },
    [toggleObjectMods, t],
  );

  const handleBulkDisable = useCallback(
    async (ids: Set<string>) => {
      for (const id of ids) await toggleObjectMods(id, false, true);

      const toastMsg = t(
        ids.size === 1 ? 'objects:toasts.disabled_one' : 'objects:toasts.disabled_other',
        {
          count: ids.size,
        },
      );

      toast.success(toastMsg);
    },
    [toggleObjectMods, t],
  );

  const handleBulkAddTags = useCallback(
    async (ids: Set<string>, tagsToAdd: string[]) => {
      for (const id of ids) {
        const obj = objects.find((o) => o.id === id);
        if (!obj) continue;
        const existing: string[] = (() => {
          try {
            return JSON.parse(obj.tags || '[]');
          } catch {
            return [];
          }
        })();
        const merged = [...new Set([...existing, ...tagsToAdd])];
        try {
          await commands.updateObject({
            id,
            updates: createObjectUpdate({ tags: merged }),
          });
        } catch (e) {
          console.error('Bulk add tags failed for', id, e);
        }
      }
      queryClient.invalidateQueries({ queryKey: ['objects'] });

      const count = ids.size;
      const displayNames = Array.from(ids).map((id) => {
        const obj = objects.find((o) => o.id === id);
        return obj ? obj.name : id;
      });

      const itemsStr =
        count <= 4
          ? displayNames.join(', ')
          : `${displayNames.slice(0, 4).join(', ')} + ${count - 4} others`;

      toast.success(t('objects:toasts.tags_added', { count: tagsToAdd.length, items: itemsStr }));
    },
    [objects, queryClient, t],
  );

  const handleBulkRemoveTags = useCallback(
    async (ids: Set<string>, tagsToRemove: string[]) => {
      const removeSet = new Set(tagsToRemove);
      for (const id of ids) {
        const obj = objects.find((o) => o.id === id);
        if (!obj) continue;
        const existing: string[] = (() => {
          try {
            return JSON.parse(obj.tags || '[]');
          } catch {
            return [];
          }
        })();
        const filtered = existing.filter((t) => !removeSet.has(t));
        try {
          await commands.updateObject({
            id,
            updates: createObjectUpdate({ tags: filtered }),
          });
        } catch (e) {
          console.error('Bulk remove tags failed for', id, e);
        }
      }
      queryClient.invalidateQueries({ queryKey: ['objects'] });

      const count = ids.size;
      const displayNames = Array.from(ids).map((id) => {
        const obj = objects.find((o) => o.id === id);
        return obj ? obj.name : id;
      });

      const itemsStr =
        count <= 4
          ? displayNames.join(', ')
          : `${displayNames.slice(0, 4).join(', ')} + ${count - 4} others`;

      toast.success(
        t('objects:toasts.tags_removed', { count: tagsToRemove.length, items: itemsStr }),
      );
    },
    [objects, queryClient, t],
  );

  const handleBulkAutoOrganize = useCallback(
    async (ids: Set<string>) => {
      if (!activeGame) return;
      try {
        const selectedObjects = objects.filter((o) => ids.has(o.id));
        const responses = await Promise.all(
          selectedObjects.map((obj) =>
            commands.listModFolders({
              gameId: activeGame.id,
              modsPath: activeGame.mod_path,
              subPath: obj.folder_path,
              objectId: null,
            }),
          ),
        );
        const allModPaths = responses.flatMap((r) => r.children.map((c) => c.path));

        if (allModPaths.length === 0) {
          toast.info(t('objects:auto_organize.toast_none'));
          return;
        }

        const dbJson = await scanService.getMasterDb(activeGame.game_type as unknown as GameType);
        const result = await commands.autoOrganizeMods({
          paths: allModPaths,
          targetRoot: activeGame.mod_path,
          dbJson,
        });

        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });

        const moved = result.success.length;
        const failed = result.failures.length;
        if (moved > 0) toast.success(t('objects:auto_organize.toast_success', { count: moved }));
        if (failed > 0) toast.error(t('objects:auto_organize.toast_failed', { count: failed }));
      } catch (e) {
        console.error('Auto-organize failed:', e);
        toast.error(t('objects:auto_organize.toast_error', { error: String(e) }));
      }
    },
    [activeGame, objects, queryClient, t],
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
        queryClient.invalidateQueries({ queryKey: ['objects'] });
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
    [activeGame, objects, queryClient, t],
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
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        toast.success(
          t(safe ? 'objects:toasts.mark_safe' : 'objects:toasts.mark_unsafe', {
            count: ids.size,
          }),
        );
      } catch (e) {
        toast.error(t('objects:edit_modal.error_message', { error: String(e) }));
      }
    },
    [activeGame, objects, queryClient, t],
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
    handleBulkAutoOrganize,
    handleBulkFavorite,
    handleBulkSafe,
  };
}
