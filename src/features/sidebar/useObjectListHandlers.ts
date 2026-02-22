/**
 * useObjectListHandlers — action handlers extracted from useObjectListLogic
 * to keep the orchestrator hook under 350 lines.
 */

import { useState, useMemo, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { mkdir, exists, remove } from '@tauri-apps/plugin-fs';
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
import { classifyDroppedPaths } from './dropUtils';

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

  const handleCloseScanReview = useCallback(async () => {
    if (scanReview.isCommitting) return;

    // Cleanup temp folder if it exists
    if (activeGame) {
      const tempPath = `${activeGame.mod_path}\\.emmm2_temp`;
      try {
        if (await exists(tempPath)) {
          await remove(tempPath, { recursive: true });
        }
      } catch (e) {
        console.error('Failed to cleanup temp folder:', e);
      }
    }

    setScanReview({ open: false, items: [], masterDbEntries: [], isCommitting: false });
  }, [scanReview.isCommitting, activeGame]);

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

  // Category names from schema
  const categoryNames = useMemo(
    () =>
      schema?.categories.map((c) => ({
        name: c.name,
        label: c.label,
      })) ?? [],
    [schema],
  );

  // US-3.Z: Zone-aware DnD handlers

  /** Drop on specific object row — move items into that object's folder */
  const handleDropOnItem = useCallback(
    async (objectId: string, paths: string[]) => {
      if (!activeGame || paths.length === 0) return;

      const obj = objects.find((o) => o.id === objectId);
      if (!obj) {
        toast.error('Could not find target object.');
        return;
      }

      toast.info(`Moving ${paths.length} item(s) to ${obj.name}...`);

      // Suppress watcher during file operations to avoid "External Deletion" dialogs
      await invoke('set_watcher_suppression_cmd', { suppressed: true });
      try {
        const classified = classifyDroppedPaths(paths);

        // Extract archives first, then ingest the extracted folders + non-archive items
        const pathsToIngest: string[] = [
          ...classified.folders,
          ...classified.iniFiles,
          ...classified.images,
        ];

        const objectFolderPath = `${activeGame.mod_path}\\${obj.name}`;

        for (const archivePath of classified.archives) {
          const result = await scanService.extractArchive(archivePath, objectFolderPath);
          if (result.success) {
            pathsToIngest.push(result.dest_path);
          } else {
            toast.error(`Failed to extract: ${result.error ?? 'Unknown error'}`);
          }
        }

        if (pathsToIngest.length === 0) {
          toast.info('No items to import after extraction.');
          return;
        }

        // Use import_mods_from_paths with the object's target directory
        const result = await invoke<{
          success: string[];
          failures: { path: string; error: string }[];
        }>('import_mods_from_paths', {
          paths: pathsToIngest,
          targetDir: objectFolderPath,
          strategy: 'Raw',
          dbJson: null,
        });

        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });

        const movedCount = result.success.length;
        const failCount = result.failures.length;
        if (movedCount > 0) {
          toast.success(
            `Moved ${movedCount} item(s) to ${obj.name}${failCount > 0 ? `, ${failCount} failed` : ''}`,
          );
        } else if (failCount > 0) {
          toast.error(`Failed to move items: ${result.failures[0].error}`);
        }
      } catch (e) {
        console.error('Drop on item failed:', e);
        toast.error('Failed to import dropped items');
      } finally {
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
      }
    },
    [activeGame, objects, queryClient],
  );

  /** Drop on Auto Organize zone — extract archives → scan → open review */
  const handleDropAutoOrganize = useCallback(
    async (paths: string[]) => {
      if (!activeGame) return;

      const classified = classifyDroppedPaths(paths);

      toast.info('Preparing Auto Organize...');

      // Suppress watcher during file operations
      await invoke('set_watcher_suppression_cmd', { suppressed: true });
      try {
        const tempPath = `${activeGame.mod_path}\\.emmm2_temp`;
        if (!(await exists(tempPath))) {
          await mkdir(tempPath, { recursive: true });
        }

        // Step 1: Extract archives to temp dir
        const folderPaths: string[] = [...classified.folders]; // These will be scanned in-place
        const tempPaths: string[] = [];

        for (const archivePath of classified.archives) {
          const result = await scanService.extractArchive(archivePath, tempPath);
          if (result.success) {
            folderPaths.push(result.dest_path);
            tempPaths.push(result.dest_path);
          } else {
            toast.error(`Failed to extract: ${result.error ?? 'Unknown error'}`);
          }
        }

        // We only pass remaining non-folder files (ini, images) to ingest_dropped_folders.
        // We do NOT pass classified.folders here, because they are scanned in-place
        // and shouldn't be permanently moved until user clicks Confirm.
        const looseFiles = [...classified.iniFiles, ...classified.images];
        if (looseFiles.length > 0) {
          const ingestResult = await invoke<{
            moved: string[];
            skipped: string[];
            not_dirs: string[];
            sync: { new_mods: number; new_objects: number };
          }>('ingest_dropped_folders', {
            paths: looseFiles,
            modsPath: tempPath, // Put loose files inside temp
            gameId: activeGame.id,
            gameName: activeGame.name,
            gameType: activeGame.game_type,
          });
          // Add ingested paths to what we scan
          folderPaths.push(...ingestResult.moved);
          tempPaths.push(...ingestResult.moved);
        }

        // Step 3: Run the full scan preview (triggers Deep Matcher)
        setIsSyncing(true);
        const previewItemsRaw = await scanService.scanPreview(
          activeGame.id,
          activeGame.game_type,
          activeGame.mod_path,
          undefined,
          folderPaths, // Pass exactly the paths we want to scan
        );

        // Mark items inside the temp folders so they can be moved on commit
        const previewItems = previewItemsRaw.map((item) => ({
          ...item,
          moveFromTemp: tempPaths.includes(item.folderPath),
        }));

        // Load MasterDB entries for override search
        const dbJson = await scanService.getMasterDb(activeGame.game_type);
        let masterEntries: {
          name: string;
          object_type: string;
          tags: string[];
          metadata: Record<string, unknown> | null;
          thumbnail_path: string | null;
        }[] = [];
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

        setScanReview({
          open: true,
          items: previewItems,
          masterDbEntries: masterEntries,
          isCommitting: false,
        });
      } catch (e) {
        console.error('Auto organize failed:', e);
        toast.error(`Auto organize failed: ${e instanceof Error ? e.message : String(e)}`);
      } finally {
        setIsSyncing(false);
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
      }
    },
    [activeGame],
  );

  /**
   * Drop on "Append as New Object" zone — returns paths to the caller.
   * The actual create flow is handled by CreateObjectModal with pendingPaths.
   * This is a no-op handler; the ObjectList orchestrator handles the UI state.
   */
  const handleDropNewObject = useCallback((_paths: string[]) => {
    // Intentionally empty — ObjectList.tsx handles opening CreateObjectModal
    // with pendingPaths. This exists so useObjectListLogic can export it.
  }, []);

  // Enable/Disable Object by toggling its physical folder directly
  const handleEnableObject = useCallback(
    async (objectId: string) => {
      if (!activeGame) return;
      const obj = objects.find((o) => o.id === objectId);
      if (!obj) return;
      try {
        // The disabled folder is at modsPath/DISABLED objectName
        const disabledPath = `${activeGame.mod_path}\\DISABLED ${obj.name}`;
        await invoke('toggle_mod', { path: disabledPath, enable: true });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        toast.success(`Enabled ${obj.name}`);
      } catch (e) {
        console.error('Failed to enable object:', e);
        toast.error('Failed to enable object');
      }
    },
    [activeGame, objects, queryClient],
  );

  const handleRevealInExplorer = useCallback(
    async (objectId: string) => {
      if (!activeGame) return;
      const obj = objects.find((o) => o.id === objectId);
      try {
        await invoke('reveal_object_in_explorer', {
          objectId,
          modsPath: activeGame.mod_path,
          objectName: obj?.name ?? objectId,
        });
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        toast.error(msg);
        // Refresh data in case stale entries were cleaned up
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      }
    },
    [activeGame, objects, queryClient],
  );

  const handleDisableObject = useCallback(
    async (objectId: string) => {
      if (!activeGame) return;
      const obj = objects.find((o) => o.id === objectId);
      if (!obj) return;
      try {
        // The enabled folder is at modsPath/objectName
        const enabledPath = `${activeGame.mod_path}\\${obj.name}`;
        await invoke('toggle_mod', { path: enabledPath, enable: false });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['objects'] });
        toast.success(`Disabled ${obj.name}`);
      } catch (e) {
        console.error('Failed to disable object:', e);
        toast.error('Failed to disable object');
      }
    },
    [activeGame, objects, queryClient],
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
    handleRevealInExplorer,
    handleEnableObject,
    handleDisableObject,
    categoryNames,
    handleDropOnItem,
    handleDropAutoOrganize,
    handleDropNewObject,
  };
}
