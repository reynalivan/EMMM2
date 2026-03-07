/**
 * useObjHandlersCrud — CRUD, pin/favorite, object toggle, and category handlers.
 * Extracted from useObjectListHandlers for SRP.
 */

import { useState, useMemo, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { join } from '@tauri-apps/api/path';
import { useToggleMod, useDeleteMod, type ModFolder } from '../../hooks/useFolders';
import { useDeleteObject, useUpdateObject } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import { toast } from '../../stores/useToastStore';
import { useAppStore } from '../../stores/useAppStore';
import { toggleDisabledInPath } from '../../lib/disabledPrefix';
import { syncExplorerAfterRename } from './objHandlersHelpers';
import type { ObjectSummary, GameSchema } from '../../types/object';

interface CrudDeps {
  objects: ObjectSummary[];
  folders: ModFolder[];
  schema: GameSchema | undefined;
}

export function useObjHandlersCrud({ objects, folders, schema }: CrudDeps) {
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();
  const toggleMod = useToggleMod();
  const deleteMod = useDeleteMod();
  const deleteObjectMutation = useDeleteObject();
  const updateObject = useUpdateObject();

  // ── State ────────────────────────────────────────────────────────
  const [deleteDialog, setDeleteDialog] = useState<{
    open: boolean;
    path: string;
    name: string;
    itemCount: number;
  }>({ open: false, path: '', name: '', itemCount: 0 });

  const [editObject, setEditObject] = useState<ObjectSummary | null>(null);

  const [deleteObjectDialog, setDeleteObjectDialog] = useState<{
    open: boolean;
    id: string;
    name: string;
  }>({ open: false, id: '', name: '' });

  const isTogglingObjectRef = useRef(false);

  // ── Toggle / Open / Delete ───────────────────────────────────────
  const handleToggle = useCallback(
    (path: string, currentEnabled: boolean) => {
      if (!activeGame?.id) return;
      toggleMod.mutate(
        { path, enable: !currentEnabled, gameId: activeGame.id },
        {
          onSuccess: (newPath) => {
            if (activeGame.mod_path) syncExplorerAfterRename(activeGame.mod_path, path, newPath);
          },
        },
      );
    },
    [toggleMod, activeGame],
  );

  const handleOpen = async (path: string) => {
    try {
      await invoke('open_in_explorer', { path });
    } catch (e) {
      console.error('Failed to open explorer:', e);
    }
  };

  const handleDelete = useCallback(
    async (path: string) => {
      try {
        const info = await invoke<{
          path: string;
          name: string;
          item_count: number;
          is_empty: boolean;
        }>('pre_delete_check', { path });

        if (info.is_empty) {
          deleteMod.mutate({ path, gameId: activeGame?.id });
        } else {
          setDeleteDialog({ open: true, path, name: info.name, itemCount: info.item_count });
        }
      } catch (e) {
        console.error('Failed to check/delete mod:', e);
      }
    },
    [activeGame, deleteMod],
  );

  const confirmDelete = useCallback(() => {
    deleteMod.mutate({ path: deleteDialog.path, gameId: activeGame?.id });
    setDeleteDialog({ open: false, path: '', name: '', itemCount: 0 });
  }, [deleteDialog.path, activeGame, deleteMod]);

  // ── Delete Object ────────────────────────────────────────────────
  const handleDeleteObject = useCallback(
    (id: string) => {
      const obj = objects.find((o) => o.id === id);
      if (!obj) return;
      setDeleteObjectDialog({ open: true, id: obj.id, name: obj.name });
    },
    [objects],
  );

  const confirmDeleteObject = useCallback(async () => {
    const { id, name } = deleteObjectDialog;
    setDeleteObjectDialog({ open: false, id: '', name: '' });
    try {
      await deleteObjectMutation.mutateAsync(id);
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      toast.success(`Deleted "${name}" successfully.`);
    } catch (e) {
      console.error('Failed to delete object:', e);
      const msg = e instanceof Error ? e.message : String(e);
      toast.error(`Failed to delete "${name}": ${msg}`);
    }
  }, [deleteObjectDialog, deleteObjectMutation, queryClient]);

  // ── Edit ─────────────────────────────────────────────────────────
  const handleEdit = useCallback(
    (id: string) => {
      const obj = objects.find((o) => o.id === id);
      if (obj) setEditObject(obj);
    },
    [objects],
  );

  // ── Pin / Favorite / Move Category ───────────────────────────────
  const handlePin = useCallback(
    async (id: string) => {
      try {
        const obj = objects.find((o) => o.id === id);
        if (obj) {
          await invoke('pin_object', { id, pin: !obj.is_pinned });
          queryClient.invalidateQueries({ queryKey: ['objects'] });
        }
      } catch (e) {
        console.error('Failed to pin object:', e);
      }
    },
    [objects, queryClient],
  );

  const handleFavorite = useCallback(
    async (pathOrId: string) => {
      try {
        const folder = folders.find((f) => f.path === pathOrId);
        if (folder && activeGame?.id) {
          await invoke('toggle_favorite', {
            gameId: activeGame.id,
            folderPath: folder.path,
            favorite: !folder.is_favorite,
          });
          queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        }
      } catch (e) {
        console.error('Failed to favorite:', e);
      }
    },
    [folders, queryClient, activeGame?.id],
  );

  const handleMoveCategory = useCallback(
    async (id: string, category: string, itemType: 'object' | 'folder') => {
      if (!activeGame) return;
      try {
        if (itemType === 'folder') {
          await invoke('set_mod_category', {
            gameId: activeGame.id,
            folderPath: id,
            category,
          });
        } else {
          await updateObject.mutateAsync({ id, updates: { object_type: category } });
          const response = await invoke<import('../../types/mod').FolderGridResponse>(
            'list_mod_folders',
            {
              gameId: activeGame.id,
              modsPath: activeGame.mod_path,
              subPath: undefined,
              objectId: id,
            },
          );
          for (const f of response.children) {
            await invoke('set_mod_category', {
              gameId: activeGame.id,
              folderPath: f.path,
              category,
            });
          }
        }
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      } catch (e) {
        console.error('Failed to move category:', e);
      }
    },
    [activeGame, queryClient, updateObject],
  );

  const categoryNames = useMemo(
    () => schema?.categories.map((c) => ({ name: c.name, label: c.label })) ?? [],
    [schema],
  );

  // ── Object Toggle (Enable/Disable root folder) ───────────────────
  const toggleObjectMods = useCallback(
    async (objectId: string, enable: boolean) => {
      if (!activeGame || isTogglingObjectRef.current) return;

      const obj = objects.find((o) => o.id === objectId);
      if (!obj) return;

      const label = enable ? 'Enable' : 'Disable';
      isTogglingObjectRef.current = true;

      const prevObjectQueries = queryClient.getQueriesData<ObjectSummary[]>({
        queryKey: ['objects', 'list'],
      });
      const prevSubPath = useAppStore.getState().explorerSubPath;

      try {
        const targetPath = await join(activeGame.mod_path, obj.folder_path);

        queryClient.setQueriesData<ObjectSummary[]>({ queryKey: ['objects', 'list'] }, (old) => {
          if (!old) return old;
          return old.map((o) => {
            if (o.id !== objectId) return o;
            return {
              ...o,
              folder_path: toggleDisabledInPath(o.folder_path, enable),
              enabled_count: enable ? o.mod_count : 0,
              is_object_disabled: !enable,
            };
          });
        });

        const newPath = await toggleMod.mutateAsync({
          path: targetPath,
          enable,
          gameId: activeGame.id,
        });

        if (activeGame.mod_path) syncExplorerAfterRename(activeGame.mod_path, targetPath, newPath);
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      } catch (e) {
        for (const [key, data] of prevObjectQueries) {
          queryClient.setQueryData(key, data);
        }
        const curSub = useAppStore.getState().explorerSubPath;
        if (curSub !== prevSubPath && prevSubPath) {
          useAppStore.getState().setExplorerSubPath(prevSubPath);
          useAppStore.getState().setCurrentPath(prevSubPath.split('/'));
        }
        console.error(`Failed to ${label.toLowerCase()} object:`, e);
        const errStr = String(e);
        if (errStr.includes('"type":"RenameConflict"')) {
          try {
            const conflict = JSON.parse(errStr);
            useAppStore.getState().openConflictDialog(conflict);
            return;
          } catch {
            /* parse failed, fall through */
          }
        }
        toast.error(`Failed to ${label.toLowerCase()} object`);
      } finally {
        isTogglingObjectRef.current = false;
      }
    },
    [activeGame, objects, queryClient, toggleMod],
  );

  const handleEnableObject = useCallback(
    (objectId: string) => toggleObjectMods(objectId, true),
    [toggleObjectMods],
  );

  const handleDisableObject = useCallback(
    (objectId: string) => toggleObjectMods(objectId, false),
    [toggleObjectMods],
  );

  const handleRevealInExplorer = useCallback(
    async (objectId: string) => {
      if (!activeGame) return;
      const obj = objects.find((o) => o.id === objectId);
      try {
        await invoke('reveal_object_in_explorer', {
          objectId,
          modsPath: activeGame.mod_path,
          objectName: obj?.folder_path ?? objectId,
        });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        toast.error(msg);
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      }
    },
    [activeGame, objects, queryClient],
  );

  return {
    deleteDialog,
    setDeleteDialog,
    editObject,
    setEditObject,
    deleteObjectDialog,
    setDeleteObjectDialog,
    handleToggle,
    handleOpen,
    handleDelete,
    confirmDelete,
    handleDeleteObject,
    confirmDeleteObject,
    handleEdit,
    handlePin,
    handleFavorite,
    handleMoveCategory,
    categoryNames,
    toggleObjectMods,
    handleEnableObject,
    handleDisableObject,
    handleRevealInExplorer,
  };
}
