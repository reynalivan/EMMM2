import { useState, useMemo, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { commands } from '../../lib/bindings';
import { join } from '@tauri-apps/api/path';
import { useToggleMod, useDeleteMod, type ModFolder } from '../../hooks/useFolders';
import { useDeleteObject, useUpdateObject } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import { toast } from '../../stores/useToastStore';
import { useAppStore } from '../../stores/useAppStore';
import { toggleDisabledInPath } from '../../lib/disabledPrefix';
import { syncExplorerAfterRename } from './objHandlersHelpers';
import { useTranslation } from 'react-i18next';
import type { ObjectSummary, GameSchema } from '../../types/object';

interface CrudDeps {
  objects: ObjectSummary[];
  folders: ModFolder[];
  schema: GameSchema | undefined;
}

export function useObjHandlersCrud({ objects, folders, schema }: CrudDeps) {
  const { t } = useTranslation(['objects', 'common']);
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

  const [forceDeleteObjectDialog, setForceDeleteObjectDialog] = useState<{
    open: boolean;
    id: string;
    name: string;
    count: number;
  }>({ open: false, id: '', name: '', count: 0 });

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
    if (!activeGame?.id) return;
    try {
      await commands.openInExplorer({ gameId: activeGame.id, path });
    } catch (e) {
      console.error('Failed to open explorer:', e);
    }
  };

  const handleDelete = useCallback(
    async (path: string) => {
      try {
        const info = await commands.preDeleteCheck({ path });

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
    try {
      await deleteObjectMutation.mutateAsync({ id, force: false });
      setDeleteObjectDialog({ open: false, id: '', name: '' });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      toast.success(t('create_modal.success_message', { name }));
    } catch (e: unknown) {
      setDeleteObjectDialog({ open: false, id: '', name: '' });
      const errObj = e as Record<string, unknown>;
      const errStr = String(errObj?.message ?? e);
      let count = 0;
      let hasModsError = false;

      try {
        const payload = typeof e === 'string' ? JSON.parse(e) : e;
        if (payload && typeof payload === 'object' && 'ObjectHasMods' in payload) {
          hasModsError = true;
          count = Number(payload.ObjectHasMods);
        }
      } catch {
        if (errStr.includes('ObjectHasMods') || errStr.includes('Object has')) {
          hasModsError = true;
          const match = errStr.match(/\d+/);
          count = match ? parseInt(match[0], 10) : 1;
        }
      }

      if (hasModsError) {
        setForceDeleteObjectDialog({ open: true, id, name, count });
      } else {
        console.error('Failed to delete object:', e);
        toast.error(t('create_modal.error_message', { error: errStr }));
      }
    }
  }, [deleteObjectDialog, deleteObjectMutation, queryClient, t]);

  const confirmForceDeleteObject = useCallback(async () => {
    const { id, name } = forceDeleteObjectDialog;
    setForceDeleteObjectDialog({ open: false, id: '', name: '', count: 0 });
    try {
      await deleteObjectMutation.mutateAsync({ id, force: true });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      toast.success(t('create_modal.success_message', { name }));
    } catch (e: unknown) {
      console.error('Failed to force delete object:', e);
      const errObj = e as Record<string, unknown>;
      toast.error(t('create_modal.error_message', { error: String(errObj?.message ?? e) }));
    }
  }, [forceDeleteObjectDialog, deleteObjectMutation, queryClient, t]);

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
          await commands.pinObject({ id, pin: !obj.is_pinned });
          queryClient.invalidateQueries({ queryKey: ['objects'] });
          toast.success(
            t(obj.is_pinned ? 'toasts.pin_removed_one' : 'toasts.pin_added_one', { count: 1 }),
          );
        }
      } catch (e) {
        console.error('Failed to pin object:', e);
      }
    },
    [objects, queryClient, t],
  );

  const handleFavorite = useCallback(
    async (pathOrId: string) => {
      try {
        const folder = folders.find((f) => f.path === pathOrId);
        if (folder && activeGame?.id) {
          await commands.toggleFavorite({
            gameId: activeGame.id,
            folderPath: folder.path,
            favorite: !folder.is_favorite,
          });
          queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
          toast.success(
            t(folder.is_favorite ? 'toasts.favorite_removed_one' : 'toasts.favorite_added_one', {
              count: 1,
            }),
          );
        }
      } catch (e) {
        console.error('Failed to favorite:', e);
      }
    },
    [folders, queryClient, activeGame?.id, t],
  );

  const handleMoveCategory = useCallback(
    async (id: string, category: string, itemType: 'object' | 'folder') => {
      if (!activeGame) return;
      try {
        if (itemType === 'folder') {
          await commands.setModCategory({
            gameId: activeGame.id,
            folderPath: id,
            category,
          });
        } else {
          await updateObject.mutateAsync({ id, updates: { object_type: category } });
          const obj = objects.find((o) => o.id === id);
          if (!obj) return;
          const response = await commands.listModFolders({
            gameId: activeGame.id,
            modsPath: activeGame.mod_path,
            subPath: obj.folder_path,
            objectId: obj.id,
          });
          for (const f of response.children) {
            await commands.setModCategory({
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
    [activeGame, objects, queryClient, updateObject],
  );

  const categoryNames = useMemo(
    () => schema?.categories.map((c) => ({ name: c.name, label: c.label })) ?? [],
    [schema],
  );

  // ── Object Toggle (Enable/Disable root folder) ───────────────────
  const toggleObjectMods = useCallback(
    async (objectId: string, enable: boolean, suppressToast: boolean = false) => {
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
              is_object_disabled: !enable,
            };
          });
        });

        const newPath = await toggleMod.mutateAsync({
          path: targetPath,
          enable,
          gameId: activeGame.id,
          suppressToast: true, // we handle our own localized toast
        });

        if (activeGame.mod_path) syncExplorerAfterRename(activeGame.mod_path, targetPath, newPath);
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });

        if (!suppressToast) {
          toast.success(t(enable ? 'toasts.enabled_one' : 'toasts.disabled_one', { count: 1 }));
        }
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
        toast.error(t('create_modal.error_message', { error: String(e) }));
      } finally {
        isTogglingObjectRef.current = false;
      }
    },
    [activeGame, objects, queryClient, toggleMod, t],
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
        await commands.revealObjectInExplorer({
          gameId: activeGame.id,
          objectId,
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
    forceDeleteObjectDialog,
    setForceDeleteObjectDialog,
    confirmForceDeleteObject,
  };
}
