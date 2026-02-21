/**
 * useObjectListHandlers — action handlers extracted from useObjectListLogic
 * to keep the orchestrator hook under 350 lines.
 */

import { useState, useMemo, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { useToggleMod, useDeleteMod, type ModFolder } from '../../hooks/useFolders';
import { useDeleteObject, useUpdateObject } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import {
  scanService,
  type ScanPreviewItem,
  type ConfirmedScanItem,
} from '../../services/scanService';
import { toast } from '../../stores/useToastStore';
import type { ObjectSummary, GameSchema } from '../../types/object';
import type { MatchedDbEntry } from './SyncConfirmModal';
import type { MasterDbEntry } from './ScanReviewModal';

interface HandlerDeps {
  objects: ObjectSummary[];
  folders?: ModFolder[];
  schema: GameSchema | undefined;
}

export function useObjectListHandlers({ objects, folders = [], schema }: HandlerDeps) {
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();

  // Mutation hooks
  const toggleMod = useToggleMod();
  const deleteMod = useDeleteMod();
  const deleteObjectMutation = useDeleteObject();
  const updateObject = useUpdateObject();

  // Delete confirmation dialog state (NC-3.3-02)
  const [deleteDialog, setDeleteDialog] = useState<{
    open: boolean;
    path: string;
    name: string;
    itemCount: number;
  }>({ open: false, path: '', name: '', itemCount: 0 });

  // Edit Object state (US-3.3)
  const [editObject, setEditObject] = useState<ObjectSummary | null>(null);

  // Sync state (US-3.5)
  const [isSyncing, setIsSyncing] = useState(false);

  // Scan review modal state (US-2.3)
  const [scanReview, setScanReview] = useState<{
    open: boolean;
    items: ScanPreviewItem[];
    masterDbEntries: MasterDbEntry[];
    isCommitting: boolean;
  }>({ open: false, items: [], masterDbEntries: [], isCommitting: false });

  // Single-object DB sync state
  const [syncConfirm, setSyncConfirm] = useState<{
    open: boolean;
    objectId: string;
    objectName: string;
    itemType: 'object' | 'folder';
    match: MatchedDbEntry | null;
    isLoading: boolean;
    /** Current object/folder data for diff preview */
    currentData: {
      name: string;
      object_type: string;
      metadata: Record<string, unknown> | null;
      thumbnail_path: string | null;
    } | null;
  }>({
    open: false,
    objectId: '',
    objectName: '',
    itemType: 'object',
    match: null,
    isLoading: false,
    currentData: null,
  });

  // --- Handlers ---

  const handleToggle = useCallback(
    (path: string, currentEnabled: boolean) => {
      toggleMod.mutate({ path, enable: !currentEnabled });
    },
    [toggleMod],
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
          setDeleteDialog({
            open: true,
            path,
            name: info.name,
            itemCount: info.item_count,
          });
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

  const handleDeleteObject = useCallback(
    async (id: string) => {
      const obj = objects.find((o) => o.id === id);
      if (!obj) return;
      try {
        await deleteObjectMutation.mutateAsync(id);
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      } catch (e) {
        console.error('Failed to delete object:', e);
      }
    },
    [objects, deleteObjectMutation, queryClient],
  );

  const handleEdit = useCallback(
    (id: string) => {
      const obj = objects.find((o) => o.id === id);
      if (obj) setEditObject(obj);
    },
    [objects],
  );

  const handleSync = useCallback(async () => {
    if (!activeGame || isSyncing) return;
    setIsSyncing(true);
    try {
      // Phase 1: Scan & match (no DB writes)
      const previewItems = await scanService.scanPreview(
        activeGame.id,
        activeGame.game_type,
        activeGame.mod_path,
      );

      // Load MasterDB entries for the override search
      const dbJson = await scanService.getMasterDb(activeGame.game_type);
      let masterEntries: MasterDbEntry[] = [];
      try {
        const parsed = JSON.parse(dbJson);
        if (Array.isArray(parsed)) {
          masterEntries = parsed.map((e: Record<string, unknown>) => ({
            name: String(e.name ?? ''),
            object_type: String(e.object_type ?? 'Other'),
            tags: Array.isArray(e.tags) ? (e.tags as string[]) : [],
            metadata: (e.metadata as Record<string, unknown>) ?? null,
            thumbnail_path: e.thumbnail_path ? String(e.thumbnail_path) : null,
          }));
        }
      } catch {
        // MasterDB parse failed — proceed with empty entries
      }

      // Open review modal
      setScanReview({
        open: true,
        items: previewItems,
        masterDbEntries: masterEntries,
        isCommitting: false,
      });
    } catch (e) {
      console.error('Scan preview failed:', e);
      toast.error(`Scan failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setIsSyncing(false);
    }
  }, [activeGame, isSyncing]);

  // Phase 2: Commit confirmed scan results to DB
  const handleCommitScan = useCallback(
    async (items: ConfirmedScanItem[]) => {
      if (!activeGame) return;
      setScanReview((prev) => ({ ...prev, isCommitting: true }));
      try {
        const result = await scanService.commitScan(
          activeGame.id,
          activeGame.name,
          activeGame.game_type,
          activeGame.mod_path,
          items,
        );
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });
        toast.success(
          `Sync complete: ${result.total_scanned} scanned, ${result.new_mods} new, ${result.new_objects} objects`,
        );
        setScanReview({ open: false, items: [], masterDbEntries: [], isCommitting: false });
      } catch (e) {
        console.error('Commit scan failed:', e);
        toast.error(`Commit failed: ${e instanceof Error ? e.message : String(e)}`);
        setScanReview((prev) => ({ ...prev, isCommitting: false }));
      }
    },
    [activeGame, queryClient],
  );

  const handleCloseScanReview = useCallback(() => {
    if (scanReview.isCommitting) return;
    setScanReview({ open: false, items: [], masterDbEntries: [], isCommitting: false });
  }, [scanReview.isCommitting]);

  // Single-object sync: match against MasterDB via Rust command
  const handleSyncWithDb = useCallback(
    async (id: string, name: string) => {
      if (!activeGame) return;
      // Detect if this is a folder (path) or object (UUID)
      const folderMatch = folders.find((f) => f.path === id);
      const isFolder = !!folderMatch;
      const itemType = isFolder ? 'folder' : 'object';

      // Build currentData for diff preview
      let currentData: {
        name: string;
        object_type: string;
        metadata: Record<string, unknown> | null;
        thumbnail_path: string | null;
      } | null = null;
      if (isFolder && folderMatch) {
        currentData = {
          name: folderMatch.name,
          object_type: folderMatch.category ?? '',
          metadata: folderMatch.metadata ?? null,
          thumbnail_path: folderMatch.thumbnail_path,
        };
      } else {
        const obj = objects.find((o) => o.id === id);
        if (obj)
          currentData = {
            name: obj.name,
            object_type: obj.object_type,
            metadata: null,
            thumbnail_path: obj.thumbnail_path,
          };
      }

      setSyncConfirm({
        open: true,
        objectId: id,
        objectName: name,
        itemType,
        match: null,
        isLoading: true,
        currentData,
      });
      try {
        const match = await invoke<MatchedDbEntry | null>('match_object_with_db', {
          gameType: activeGame.game_type,
          objectName: name,
        });
        setSyncConfirm((prev) => ({ ...prev, match, isLoading: false }));
      } catch (e) {
        console.error('Match failed:', e);
        setSyncConfirm((prev) => ({ ...prev, isLoading: false }));
      }
    },
    [activeGame, folders, objects],
  );

  // Apply matched DB entry to object or folder
  const handleApplySyncMatch = useCallback(
    async (match: MatchedDbEntry) => {
      const { objectId, itemType } = syncConfirm;
      if (!objectId) return;
      try {
        if (itemType === 'object') {
          // DB object: update via SQL
          await updateObject.mutateAsync({
            id: objectId,
            updates: {
              name: match.name,
              object_type: match.object_type,
              metadata: (match.metadata as Record<string, unknown>) ?? undefined,
              thumbnail_path: match.thumbnail_path ?? undefined,
            },
          });
        } else {
          // Folder: update via Rust commands (rename + category + info.json)
          let currentPath = objectId;

          // 1. Rename folder if name differs
          const folder = folders.find((f) => f.path === objectId);
          if (folder && match.name !== folder.name) {
            const result = await invoke<{ new_path: string }>('rename_mod_folder', {
              folderPath: objectId,
              newName: match.name,
            });
            currentPath = result.new_path;
          }

          // 2. Set category
          if (activeGame && match.object_type) {
            await invoke('set_mod_category', {
              gameId: activeGame.id,
              folderPath: currentPath,
              category: match.object_type,
            });
          }

          // 3. Update info.json with metadata
          if (match.metadata) {
            const metaStrings: Record<string, string> = {};
            Object.entries(match.metadata).forEach(([k, v]) => {
              if (v !== undefined && v !== null) metaStrings[k] = String(v);
            });
            await invoke('update_mod_info', {
              folderPath: currentPath,
              update: { metadata: metaStrings },
            });
          }

          // 4. Set thumbnail from MasterDB if available
          if (match.thumbnail_path) {
            await invoke('paste_thumbnail', {
              folderPath: currentPath,
              sourcePath: match.thumbnail_path,
            });
          }
        }

        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });
        setSyncConfirm({
          open: false,
          objectId: '',
          objectName: '',
          itemType: 'object',
          match: null,
          isLoading: false,
          currentData: null,
        });
      } catch (e) {
        console.error('Apply sync match failed:', e);
      }
    },
    [syncConfirm, updateObject, queryClient, folders, activeGame],
  );

  // Pin Object (Categories only)
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

  // Favorite Mod (Folders)
  const handleFavorite = useCallback(
    async (pathOrId: string) => {
      try {
        // pathOrId could be path (from folder view) or id? folder view uses path as key.
        const folder = folders.find((f) => f.path === pathOrId);

        if (folder && folder.id) {
          await invoke('toggle_favorite', { id: folder.id, favorite: !folder.is_favorite });
          queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        } else {
          console.warn('Cannot favorite - folder has no ID (not in DB yet?)');
        }
      } catch (e) {
        console.error('Failed to favorite:', e);
      }
    },
    [folders, queryClient],
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
          // Object move: 1) Update DB
          await updateObject.mutateAsync({
            id,
            updates: { object_type: category },
          });

          // Object move: 2) Retrieve and update info.json for all child mods
          const objectFolders = await invoke<{ path: string; is_enabled: boolean }[]>(
            'get_folders',
            {
              gameId: activeGame.id,
              modsPath: activeGame.mod_path,
              objectId: id,
            },
          );

          for (const f of objectFolders) {
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

  // Dynamic categories from schema (with display labels)
  const categoryNames = useMemo(() => {
    const cats: { name: string; label?: string }[] = schema?.categories ?? [
      { name: 'Character' },
      { name: 'Weapon' },
      { name: 'UI' },
      { name: 'Other' },
    ];
    return cats.map((c) => ({ name: c.name, label: c.label }));
  }, [schema]);

  // Enable/Disable Object by toggling its underlying folders
  const handleEnableObject = useCallback(
    async (objectId: string) => {
      if (!activeGame) return;
      try {
        const folders = await invoke<{ path: string; is_enabled: boolean }[]>('list_mod_folders', {
          gameId: activeGame.id,
          modsPath: activeGame.mod_path,
          subPath: null,
          objectId,
        });
        const disabledPaths = folders.filter((f) => !f.is_enabled).map((f) => f.path);
        if (disabledPaths.length === 0) return;
        await invoke('bulk_toggle_mods', { paths: disabledPaths, enable: true });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        toast.success(`Enabled ${disabledPaths.length} object folders`);
      } catch (e) {
        console.error('Failed to enable object folders:', e);
        toast.error('Failed to enable object folders');
      }
    },
    [activeGame, queryClient],
  );

  const handleDisableObject = useCallback(
    async (objectId: string) => {
      if (!activeGame) return;
      try {
        const folders = await invoke<{ path: string; is_enabled: boolean }[]>('list_mod_folders', {
          gameId: activeGame.id,
          modsPath: activeGame.mod_path,
          subPath: null,
          objectId,
        });
        const enabledPaths = folders.filter((f) => f.is_enabled).map((f) => f.path);
        if (enabledPaths.length === 0) return;
        await invoke('bulk_toggle_mods', { paths: enabledPaths, enable: false });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        toast.success(`Disabled ${enabledPaths.length} object folders`);
      } catch (e) {
        console.error('Failed to disable object folders:', e);
        toast.error('Failed to disable object folders');
      }
    },
    [activeGame, queryClient],
  );

  return {
    // Dialog state
    deleteDialog,
    setDeleteDialog,
    editObject,
    setEditObject,
    isSyncing,
    syncConfirm,
    setSyncConfirm,
    scanReview,
    handleCommitScan,
    handleCloseScanReview,

    // Handlers
    handleToggle,
    handleOpen,
    handleDelete,
    confirmDelete,
    handleDeleteObject,
    handleEdit,
    handleSync,
    handleSyncWithDb,
    handleApplySyncMatch,
    handlePin,
    handleFavorite,
    handleMoveCategory,
    handleEnableObject,
    handleDisableObject,
    categoryNames,
  };
}
