/**
 * useObjHandlersBulk — Bulk operation handlers for ObjectList.
 * Extracted from useObjectListHandlers for SRP.
 */

import { useState, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useDeleteObject } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import { type FolderGridResponse } from '../../hooks/useFolders';
import { scanService } from '../../lib/services/scanService';
import { toast } from '../../stores/useToastStore';
import type { ObjectSummary } from '../../types/object';
import type { BulkResult } from '../../types/mod';

interface BulkDeps {
  objects: ObjectSummary[];
  toggleObjectMods: (objectId: string, enable: boolean, suppressToast?: boolean) => Promise<void>;
}

export function useObjHandlersBulk({ objects, toggleObjectMods }: BulkDeps) {
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
          await deleteObjectMutation.mutateAsync(id);
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
        toast.success(`Deleted ${itemsStr}`);
      } else {
        toast.error(`Deleted ${success}, failed ${failed}.`);
      }
    },
    [deleteObjectMutation, queryClient, objects],
  );

  const handleBulkPin = useCallback(
    async (ids: Set<string>, pin: boolean) => {
      for (const id of ids) {
        try {
          await invoke('pin_object', { id, pin });
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

      const action = pin ? 'Pinned' : 'Unpinned';
      const toastMsg =
        count <= 4
          ? `${action} ${displayNames.join(', ')}`
          : `${action} ${displayNames.slice(0, 4).join(', ')} + ${count - 4} others`;

      toast.success(toastMsg);
    },
    [queryClient, objects],
  );

  const handleBulkEnable = useCallback(
    async (ids: Set<string>) => {
      for (const id of ids) await toggleObjectMods(id, true, true);

      const count = ids.size;
      const displayNames = Array.from(ids).map((id) => {
        const obj = objects.find((o) => o.id === id);
        return obj ? obj.name : id;
      });

      const toastMsg =
        count <= 4
          ? `Enabled ${displayNames.join(', ')}`
          : `Enabled ${displayNames.slice(0, 4).join(', ')} + ${count - 4} others`;

      toast.success(toastMsg);
    },
    [toggleObjectMods, objects],
  );

  const handleBulkDisable = useCallback(
    async (ids: Set<string>) => {
      for (const id of ids) await toggleObjectMods(id, false, true);

      const count = ids.size;
      const displayNames = Array.from(ids).map((id) => {
        const obj = objects.find((o) => o.id === id);
        return obj ? obj.name : id;
      });

      const toastMsg =
        count <= 4
          ? `Disabled ${displayNames.join(', ')}`
          : `Disabled ${displayNames.slice(0, 4).join(', ')} + ${count - 4} others`;

      toast.success(toastMsg);
    },
    [toggleObjectMods, objects],
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
          await invoke('update_object_cmd', { id, updates: { tags: merged } });
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

      toast.success(
        `Added ${tagsToAdd.length} tag${tagsToAdd.length !== 1 ? 's' : ''} to ${itemsStr}`,
      );
    },
    [objects, queryClient],
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
          await invoke('update_object_cmd', { id, updates: { tags: filtered } });
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
        `Removed ${tagsToRemove.length} tag${tagsToRemove.length !== 1 ? 's' : ''} from ${itemsStr}`,
      );
    },
    [objects, queryClient],
  );

  const handleBulkAutoOrganize = useCallback(
    async (ids: Set<string>) => {
      if (!activeGame) return;
      try {
        const selectedObjects = objects.filter((o) => ids.has(o.id));
        const responses = await Promise.all(
          selectedObjects.map((obj) =>
            invoke<FolderGridResponse>('list_mod_folders', {
              gameId: activeGame.id,
              modsPath: activeGame.mod_path,
              subPath: obj.folder_path,
              objectId: null,
            }),
          ),
        );
        const allModPaths = responses.flatMap((r) => r.children.map((c) => c.path));

        if (allModPaths.length === 0) {
          toast.info('No mod folders found in the selected objects.');
          return;
        }

        const dbJson = await scanService.getMasterDb(activeGame.game_type);
        const result = await invoke<BulkResult>('auto_organize_mods', {
          paths: allModPaths,
          targetRoot: activeGame.mod_path,
          dbJson,
        });

        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });

        const moved = result.success.length;
        const failed = result.failures.length;
        if (moved > 0) toast.success(`Auto-organized ${moved} mod(s).`);
        if (failed > 0) toast.error(`Failed to organize ${failed} mod(s).`);
      } catch (e) {
        console.error('Auto-organize failed:', e);
        toast.error(`Auto-organize failed: ${e instanceof Error ? e.message : String(e)}`);
      }
    },
    [activeGame, objects, queryClient],
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
  };
}
