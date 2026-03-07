/**
 * useObjectListHandlers — action handlers extracted from useObjectListLogic
 * to keep the orchestrator hook under 350 lines.
 */

import { useState, useMemo, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { basename, join } from '@tauri-apps/api/path';
import { mkdir, exists, remove } from '@tauri-apps/plugin-fs';
import {
  useToggleMod,
  useDeleteMod,
  type ModFolder,
  type FolderGridResponse,
} from '../../hooks/useFolders';
import { useDeleteObject, useUpdateObject } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import {
  scanService,
  type ScanPreviewItem,
  type ConfirmedScanItem,
} from '../../lib/services/scanService';
import { toast } from '../../stores/useToastStore';
import type { ObjectSummary, GameSchema } from '../../types/object';
import type { BulkResult } from '../../types/mod';
import type { MatchedDbEntry } from './SyncConfirmModal';
import type { MasterDbEntry } from './ScanReviewModal';
import { classifyDroppedPaths } from './dropUtils';
import { useAppStore } from '../../stores/useAppStore';
import { toggleDisabledInPath } from '../../lib/disabledPrefix';

// ── Shared helper: sync explorer navigation after a folder rename ──────
function syncExplorerAfterRename(modPath: string, oldPath: string, newPath: string): void {
  const { explorerSubPath, setExplorerSubPath, setCurrentPath } = useAppStore.getState();
  if (!explorerSubPath) return;

  const clean = (p: string) => p.replace(/\\/g, '/');
  const cleanMod = clean(modPath);
  const cleanOld = clean(oldPath);
  const cleanNew = clean(newPath);
  const currentAbs = `${cleanMod}/${clean(explorerSubPath)}`;

  if (currentAbs === cleanOld || currentAbs.startsWith(cleanOld + '/')) {
    const updated =
      currentAbs === cleanOld ? cleanNew : currentAbs.replace(cleanOld + '/', cleanNew + '/');
    let sub = updated.substring(cleanMod.length);
    if (sub.startsWith('/')) sub = sub.substring(1);
    if (sub && sub !== explorerSubPath) {
      setExplorerSubPath(sub);
      setCurrentPath(sub.split('/'));
    }
  }
}

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

  // Archive modal state (US-2.1)
  const [archiveModal, setArchiveModal] = useState<{
    open: boolean;
    archives: import('../../types/scanner').ArchiveInfo[];
    isExtracting: boolean;
    error: string | null;
    pendingDropContext: {
      type: 'item' | 'auto-organize' | 'new-object';
      pathsToIngest: string[]; // For item drop and new-object
      targetFolder?: string; // For item drop and new-object
      targetObjectId?: string; // For item drop and new-object
      baseFolderPaths?: string[]; // For auto-organize
      baseLooseFiles?: string[]; // For auto-organize
    } | null;
  }>({
    open: false,
    archives: [],
    isExtracting: false,
    error: null,
    pendingDropContext: null,
  });

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

  const [deleteObjectDialog, setDeleteObjectDialog] = useState<{
    open: boolean;
    id: string;
    name: string;
  }>({ open: false, id: '', name: '' });

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
        const errMsg = e instanceof Error ? e.message : String(e);
        if (errMsg.includes('DUPLICATE|')) {
          const dest = errMsg.split('DUPLICATE|')[1] || '';
          toast.error(
            `Destination exists: ${dest}. Please rename the folder or skip it to continue.`,
          );
        } else {
          toast.error(`Commit failed: ${errMsg}`);
        }
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
          // Object move: 1) Update DB
          await updateObject.mutateAsync({
            id,
            updates: { object_type: category },
          });

          // Object move: 2) Retrieve and update info.json for all child mods
          const response = await invoke<import('../../types/mod').FolderGridResponse>(
            'list_mod_folders',
            {
              gameId: activeGame.id,
              modsPath: activeGame.mod_path,
              subPath: undefined,
              objectId: id,
            },
          );
          const objectFolders = response.children;

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

  /** Common helper to launch ArchiveModal if archives are present */
  const handleArchivesInteractively = useCallback(
    async (archivePaths: string[], context: typeof archiveModal.pendingDropContext) => {
      // Analyze archives to get size and has_ini flag
      const archiveInfos = await Promise.all(
        archivePaths.map(async (path) => {
          try {
            const analysis = await invoke<import('../../types/scanner').ArchiveAnalysis>(
              'analyze_archive_cmd',
              {
                archivePath: path,
              },
            );
            const name = await basename(path);
            return {
              path,
              name,
              extension: name.split('.').pop() || '',
              size_bytes: analysis.total_size_bytes ?? 0,
              has_ini: analysis.has_ini,
              file_count: analysis.file_count ?? 1,
              is_encrypted: analysis.is_encrypted || false,
            } as import('../../types/scanner').ArchiveInfo;
          } catch (e) {
            console.error(`Failed to analyze archive ${path}:`, e);
            const name = await basename(path);
            return {
              path,
              name,
              extension: name.split('.').pop() || '',
              size_bytes: 0,
              has_ini: false,
              file_count: 0,
              is_encrypted: false,
            } as import('../../types/scanner').ArchiveInfo;
          }
        }),
      );

      setArchiveModal({
        open: true,
        archives: archiveInfos,
        isExtracting: false,
        error: null,
        pendingDropContext: context,
      });
    },
    [archiveModal],
  );

  /** Handles the actual extraction and resuming the drop flow, called from ArchiveModal */
  const handleArchiveExtractSubmit = useCallback(
    async (selectedPaths: string[], passwords: Record<string, string>, _overwrite?: boolean) => {
      const { pendingDropContext, archives } = archiveModal;
      if (!pendingDropContext || !activeGame) return;

      setArchiveModal((prev) => ({ ...prev, isExtracting: true, error: null }));

      // Suppress watcher during file operations
      await invoke('set_watcher_suppression_cmd', { suppressed: true });

      try {
        const extractedFolders: string[] = [];

        // Determine extraction target Temp for auto-organize, Target for item
        const extractTarget =
          pendingDropContext.type === 'item'
            ? pendingDropContext.targetFolder!
            : `${activeGame.mod_path}\\.emmm2_temp`;

        if (pendingDropContext.type === 'auto-organize' && !(await exists(extractTarget))) {
          await mkdir(extractTarget, { recursive: true });
        }

        // Split into non-encrypted and encrypted
        const nonEncrypted = selectedPaths.filter((p) => {
          const info = archives.find((a) => a.path === p);
          return !info?.is_encrypted;
        });
        const encrypted = selectedPaths.filter((p) => {
          const info = archives.find((a) => a.path === p);
          return !!info?.is_encrypted;
        });

        // Helper to extract a single archive
        const extractSingle = async (archivePath: string, pw?: string) => {
          const result = await scanService.extractArchive(archivePath, extractTarget, pw);
          if (result.success) {
            const paths = result.dest_paths ?? [];
            extractedFolders.push(...paths);
          } else {
            throw new Error(`Failed to extract ${await basename(archivePath)}: ${result.error}`);
          }
        };

        // Extract non-encrypted first
        for (const archivePath of nonEncrypted) {
          await extractSingle(archivePath);
        }

        // Extract encrypted with their respective passwords
        for (const archivePath of encrypted) {
          await extractSingle(archivePath, passwords[archivePath]);
        }

        // Close modal on success
        setArchiveModal((prev) => ({ ...prev, open: false, isExtracting: false }));

        // Resume flow depending on context
        if (pendingDropContext.type === 'item' || pendingDropContext.type === 'new-object') {
          const obj = objects.find((o) => o.id === pendingDropContext.targetObjectId);
          const pathsToIngest = [...pendingDropContext.pathsToIngest, ...extractedFolders];
          const isNewObject = pendingDropContext.type === 'new-object';

          if (pathsToIngest.length === 0) {
            toast.info('No items to import.');
            return;
          }

          const result = await invoke<{
            success: string[];
            failures: { path: string; error: string }[];
          }>('import_mods_from_paths', {
            paths: pathsToIngest,
            targetDir: pendingDropContext.targetFolder,
            strategy: 'Raw',
            dbJson: null,
          });

          queryClient.invalidateQueries({ queryKey: ['objects'] });
          queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
          queryClient.invalidateQueries({ queryKey: ['category-counts'] });

          const movedCount = result.success.length;
          const failCount = result.failures.length;

          if (movedCount > 0) {
            if (isNewObject) {
              toast.success(
                `Created ${obj?.name} with ${movedCount} item(s)${failCount > 0 ? `, ${failCount} failed` : ''}`,
              );
            } else {
              toast.success(
                `Moved ${movedCount} item(s)${obj ? ` to ${obj.name}` : ''}${failCount > 0 ? `, ${failCount} failed` : ''}`,
              );
            }
          } else if (failCount > 0) {
            toast.error(`Failed to move items: ${result.failures[0].error}`);
          }
        } else if (pendingDropContext.type === 'auto-organize') {
          // Auto-organize continuation
          const folderPaths = [...(pendingDropContext.baseFolderPaths || []), ...extractedFolders];
          const looseFiles = pendingDropContext.baseLooseFiles || [];

          if (looseFiles.length > 0) {
            const ingestResult = await invoke<{
              moved: string[];
              skipped: string[];
              not_dirs: string[];
              sync: { new_mods: number; new_objects: number };
            }>('ingest_dropped_folders', {
              paths: looseFiles,
              modsPath: extractTarget,
              gameId: activeGame.id,
              gameName: activeGame.name,
              gameType: activeGame.game_type,
            });
            folderPaths.push(...ingestResult.moved);
          }

          setIsSyncing(true);
          const previewItemsRaw = await scanService.scanPreview(
            activeGame.id,
            activeGame.game_type,
            activeGame.mod_path,
            undefined,
            folderPaths,
          );

          const previewItems = previewItemsRaw.map((item) => ({
            ...item,
            moveFromTemp: folderPaths.includes(item.folderPath), // Simplified temp marking
          }));

          // Load MasterDB...
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
          } catch (e) {
            console.error('MasterDB parse failed:', e);
          }

          setScanReview({
            open: true,
            items: previewItems,
            masterDbEntries: masterEntries,
            isCommitting: false,
          });
        }
      } catch (e: unknown) {
        console.error('Extraction flow failed:', e);
        setArchiveModal((prev) => ({
          ...prev,
          isExtracting: false,
          error: e instanceof Error ? e.message : String(e),
        }));
      } finally {
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
        if (pendingDropContext.type === 'auto-organize') {
          setIsSyncing(false);
        }
      }
    },
    [archiveModal, activeGame, objects, queryClient, setArchiveModal],
  );

  /** Called when user skips extraction in the ArchiveModal */
  const handleArchiveExtractSkip = useCallback(async () => {
    const { pendingDropContext } = archiveModal;
    setArchiveModal((prev) => ({ ...prev, open: false }));

    if (!pendingDropContext || !activeGame) return;

    // Proceed with non-archive items
    if (pendingDropContext.type === 'item' || pendingDropContext.type === 'new-object') {
      const obj = objects.find((o) => o.id === pendingDropContext.targetObjectId);
      const isNewObject = pendingDropContext.type === 'new-object';
      if (pendingDropContext.pathsToIngest.length === 0) {
        toast.info('No items to import.');
        return;
      }

      try {
        await invoke('set_watcher_suppression_cmd', { suppressed: true });
        const result = await invoke<{
          success: string[];
          failures: { path: string; error: string }[];
        }>('import_mods_from_paths', {
          paths: pendingDropContext.pathsToIngest,
          targetDir: pendingDropContext.targetFolder,
          strategy: 'Raw',
          dbJson: null,
        });

        queryClient.invalidateQueries({ queryKey: ['objects'] });
        queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });

        const movedCount = result.success.length;
        const failCount = result.failures.length;

        if (movedCount > 0) {
          if (isNewObject) {
            toast.success(
              `Created ${obj?.name} with ${movedCount} item(s)${failCount > 0 ? `, ${failCount} failed` : ''}`,
            );
          } else {
            toast.success(
              `Moved ${movedCount} item(s)${obj ? ` to ${obj.name}` : ''}${failCount > 0 ? `, ${failCount} failed` : ''}`,
            );
          }
        } else if (failCount > 0) {
          toast.error(`Failed to move items: ${result.failures[0].error}`);
        }
      } catch (e) {
        console.error('Drop on item failed after skipping archives:', e);
        toast.error('Failed to import items');
      } finally {
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
      }
    } else if (pendingDropContext.type === 'auto-organize') {
      try {
        // Auto-organize continuation without archives
        const folderPaths = [...(pendingDropContext.baseFolderPaths || [])];
        const looseFiles = pendingDropContext.baseLooseFiles || [];

        if (folderPaths.length === 0 && looseFiles.length === 0) {
          toast.info('No items to auto-organize.');
          return;
        }

        setIsSyncing(true);
        await invoke('set_watcher_suppression_cmd', { suppressed: true });

        const extractTarget = `${activeGame.mod_path}\\.emmm2_temp`;
        if (looseFiles.length > 0) {
          if (!(await exists(extractTarget))) {
            await mkdir(extractTarget, { recursive: true });
          }
          const ingestResult = await invoke<{
            moved: string[];
            skipped: string[];
            not_dirs: string[];
            sync: { new_mods: number; new_objects: number };
          }>('ingest_dropped_folders', {
            paths: looseFiles,
            modsPath: extractTarget,
            gameId: activeGame.id,
            gameName: activeGame.name,
            gameType: activeGame.game_type,
          });
          folderPaths.push(...ingestResult.moved);
        }

        const previewItemsRaw = await scanService.scanPreview(
          activeGame.id,
          activeGame.game_type,
          activeGame.mod_path,
          undefined,
          folderPaths,
        );

        const previewItems = previewItemsRaw.map((item) => ({
          ...item,
          moveFromTemp: folderPaths.includes(item.folderPath),
        }));

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
        } catch (e) {
          console.error('MasterDB parse failed:', e);
        }

        setScanReview({
          open: true,
          items: previewItems,
          masterDbEntries: masterEntries,
          isCommitting: false,
        });
      } catch (e: unknown) {
        console.error('Auto-organize failed post-skip:', e);
        toast.error(`Auto organize failed: ${e instanceof Error ? e.message : String(e)}`);
      } finally {
        setIsSyncing(false);
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
      }
    }
  }, [archiveModal, activeGame, objects, queryClient, setArchiveModal]);

  /** Drop on specific object row — move items into that object's folder */
  const handleDropOnItem = useCallback(
    async (objectId: string, paths: string[]) => {
      if (!activeGame || paths.length === 0) return;

      const obj = objects.find((o) => o.id === objectId);
      if (!obj) {
        toast.error('Could not find target object.');
        return;
      }

      toast.info(`Importing item(s) to ${obj.name}...`);
      const classified = classifyDroppedPaths(paths);
      const objectFolderPath = `${activeGame.mod_path}\\${obj.folder_path}`;

      const pathsToIngest: string[] = [
        ...classified.folders,
        ...classified.iniFiles,
        ...classified.images,
      ];

      // If there are archives, delegate to ArchiveModal
      if (classified.archives.length > 0) {
        handleArchivesInteractively(classified.archives, {
          type: 'item',
          pathsToIngest,
          targetFolder: objectFolderPath,
          targetObjectId: objectId,
        });
        return;
      }

      // No archives, proceed normally
      await invoke('set_watcher_suppression_cmd', { suppressed: true });
      try {
        if (pathsToIngest.length === 0) {
          toast.info('No items to import.');
          return;
        }

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
    [activeGame, objects, queryClient, handleArchivesInteractively],
  );

  /** Drop on Auto Organize zone — extract archives → scan → open review */
  const handleDropAutoOrganize = useCallback(
    async (paths: string[]) => {
      if (!activeGame) return;

      const classified = classifyDroppedPaths(paths);
      toast.info('Preparing Auto Organize...');

      const folderPaths: string[] = [...classified.folders];
      const looseFiles = [...classified.iniFiles, ...classified.images];

      // If there are archives, delegate to ArchiveModal
      if (classified.archives.length > 0) {
        handleArchivesInteractively(classified.archives, {
          type: 'auto-organize',
          baseFolderPaths: folderPaths,
          baseLooseFiles: looseFiles,
          pathsToIngest: [], // Unused for auto-organize but required by type
        });
        return;
      }

      // Suppress watcher during file operations
      await invoke('set_watcher_suppression_cmd', { suppressed: true });
      try {
        const tempPath = `${activeGame.mod_path}\\.emmm2_temp`;
        if (looseFiles.length > 0 && !(await exists(tempPath))) {
          await mkdir(tempPath, { recursive: true });
        }

        const tempPaths: string[] = [];
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
          moveFromTemp: folderPaths.includes(item.folderPath),
        }));

        // Load MasterDB entries for override search
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
        } catch (e) {
          console.error('MasterDB parse failed in auto-organize:', e);
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
    [activeGame, handleArchivesInteractively],
  );

  /**
   * Called by CreateObjectModal after creating a DB shell to physically ingest files
   */
  const handleDropOnNewObjectSubmit = useCallback(
    async (newObjectId: string, objectName: string, paths: string[]) => {
      if (!activeGame || paths.length === 0) return;

      toast.info(`Importing item(s) to ${objectName}...`);
      const classified = classifyDroppedPaths(paths);
      // Construct physically matching folder path utilizing objectName directly
      // Note: We use objectName because for newly created objects,
      // CreateObjectModal uses input.name as folder_path unless overridden
      const objectFolderPath = `${activeGame.mod_path}\\${objectName}`;

      const pathsToIngest: string[] = [
        ...classified.folders,
        ...classified.iniFiles,
        ...classified.images,
      ];

      // Step 1: Ensure directory exists physically if not already
      try {
        if (!(await exists(objectFolderPath))) {
          await mkdir(objectFolderPath, { recursive: true });
        }
      } catch (e) {
        console.error('Failed to create object directory on disk:', e);
        toast.error('Failed to create object directory on disk.');
        return;
      }

      // Step 2: Extract & inject if we have archives
      if (classified.archives.length > 0) {
        handleArchivesInteractively(classified.archives, {
          type: 'new-object',
          pathsToIngest,
          targetFolder: objectFolderPath,
          targetObjectId: newObjectId,
        });
        return;
      }

      // Step 3: No archives, proceed normally
      await invoke('set_watcher_suppression_cmd', { suppressed: true });
      try {
        if (pathsToIngest.length === 0) {
          toast.success(`Created ${objectName} successfully (no items imported).`);
          return;
        }

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
            `Created ${objectName} with ${movedCount} item(s)${failCount > 0 ? `, ${failCount} failed` : ''}`,
          );
        } else if (failCount > 0) {
          toast.error(`Created Object but failed to move items: ${result.failures[0].error}`);
        }
      } catch (e) {
        console.error('Drop on new object failed:', e);
        toast.error('Failed to import dropped items');
      } finally {
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
      }
    },
    [activeGame, queryClient, handleArchivesInteractively],
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

  // ── Concurrency guard for object toggle ────────────────────────
  const isTogglingObjectRef = useRef(false);

  // ── Shared: toggle the Object root folder ────────────────────────
  const toggleObjectMods = useCallback(
    async (objectId: string, enable: boolean) => {
      if (!activeGame || isTogglingObjectRef.current) return;

      const obj = objects.find((o) => o.id === objectId);
      if (!obj) return;

      const label = enable ? 'Enable' : 'Disable';
      isTogglingObjectRef.current = true;

      // Snapshot for rollback — use correct prefix key ['objects','list']
      const prevObjectQueries = queryClient.getQueriesData<ObjectSummary[]>({
        queryKey: ['objects', 'list'],
      });
      const prevSubPath = useAppStore.getState().explorerSubPath;

      try {
        const targetPath = await join(activeGame.mod_path, obj.folder_path);

        // 1. Optimistic update: cache shape is ObjectSummary[] (plain array)
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

        // 2. Await backend rename
        const newPath = await toggleMod.mutateAsync({
          path: targetPath,
          enable,
          gameId: activeGame.id,
        });

        // 3. Sync explorer navigation
        if (activeGame.mod_path) syncExplorerAfterRename(activeGame.mod_path, targetPath, newPath);

        // 4. Category counts may have changed
        // Note: ['objects'] and ['mod-folders'] are already invalidated by useToggleMod.onSettled
        queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      } catch (e) {
        // Revert optimistic update — restore all matching queries
        for (const [key, data] of prevObjectQueries) {
          queryClient.setQueryData(key, data);
        }
        // Revert explorer navigation
        const curSub = useAppStore.getState().explorerSubPath;
        if (curSub !== prevSubPath && prevSubPath) {
          useAppStore.getState().setExplorerSubPath(prevSubPath);
          useAppStore.getState().setCurrentPath(prevSubPath.split('/'));
        }
        console.error(`Failed to ${label.toLowerCase()} object:`, e);
        // Detect structured RenameConflict error → open Resolve dialog
        const errStr = String(e);
        if (errStr.includes('"type":"RenameConflict"')) {
          try {
            const conflict = JSON.parse(errStr);
            useAppStore.getState().openConflictDialog(conflict);
            return;
          } catch {
            /* parse failed, fall through to generic toast */
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

  // --- Bulk action state & handlers ---
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
      if (failed === 0) {
        toast.success(`Deleted ${success} object${success !== 1 ? 's' : ''}.`);
      } else {
        toast.error(`Deleted ${success}, failed ${failed}.`);
      }
    },
    [deleteObjectMutation, queryClient],
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
      toast.success(
        `${pin ? 'Pinned' : 'Unpinned'} ${ids.size} object${ids.size !== 1 ? 's' : ''}.`,
      );
    },
    [queryClient],
  );

  const handleBulkEnable = useCallback(
    async (ids: Set<string>) => {
      for (const id of ids) {
        await toggleObjectMods(id, true);
      }
      toast.success(`Enabled ${ids.size} object${ids.size !== 1 ? 's' : ''}.`);
    },
    [toggleObjectMods],
  );

  const handleBulkDisable = useCallback(
    async (ids: Set<string>) => {
      for (const id of ids) {
        await toggleObjectMods(id, false);
      }
      toast.success(`Disabled ${ids.size} object${ids.size !== 1 ? 's' : ''}.`);
    },
    [toggleObjectMods],
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
      toast.success(
        `Added ${tagsToAdd.length} tag${tagsToAdd.length !== 1 ? 's' : ''} to ${ids.size} object${ids.size !== 1 ? 's' : ''}.`,
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
      toast.success(
        `Removed ${tagsToRemove.length} tag${tagsToRemove.length !== 1 ? 's' : ''} from ${ids.size} object${ids.size !== 1 ? 's' : ''}.`,
      );
    },
    [objects, queryClient],
  );

  const handleBulkAutoOrganize = useCallback(
    async (ids: Set<string>) => {
      if (!activeGame) return;

      try {
        // 1. Resolve mod folder paths from selected objects
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

        // 2. Get MasterDB
        const dbJson = await scanService.getMasterDb(activeGame.game_type);

        // 3. Call auto_organize_mods
        const result = await invoke<BulkResult>('auto_organize_mods', {
          paths: allModPaths,
          targetRoot: activeGame.mod_path,
          dbJson,
        });

        // 4. Invalidate and report
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
    deleteObjectDialog,
    setDeleteObjectDialog,
    confirmDeleteObject,
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
    handleDropOnNewObjectSubmit,
    archiveModal,
    handleArchivesInteractively,
    handleArchiveExtractSubmit,
    handleArchiveExtractSkip,

    // --- Bulk action handlers ---
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
