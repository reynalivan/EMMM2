/**
 * useObjHandlersScan — Scan preview, commit, and single-object DB sync.
 * Extracted from useObjectListHandlers for SRP.
 */

import { useState, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { exists, remove } from '@tauri-apps/plugin-fs';
import { useUpdateObject } from '../../hooks/useObjects';
import { useActiveGame } from '../../hooks/useActiveGame';
import {
  scanService,
  type ScanPreviewItem,
  type ConfirmedScanItem,
} from '../../lib/services/scanService';
import { toast } from '../../stores/useToastStore';
import { parseMasterDb } from './objHandlersHelpers';
import type { ObjectSummary } from '../../types/object';
import type { ModFolder } from '../../hooks/useFolders';
import type { MatchedDbEntry } from './SyncConfirmModal';
import type { MasterDbEntry } from './scanReviewHelpers';

interface ScanDeps {
  objects: ObjectSummary[];
  folders: ModFolder[];
}

export function useObjHandlersScan({ objects, folders }: ScanDeps) {
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();
  const updateObject = useUpdateObject();

  // ── State ────────────────────────────────────────────────────────
  const [isSyncing, setIsSyncing] = useState(false);

  const [scanReview, setScanReview] = useState<{
    open: boolean;
    items: ScanPreviewItem[];
    masterDbEntries: MasterDbEntry[];
    isCommitting: boolean;
  }>({ open: false, items: [], masterDbEntries: [], isCommitting: false });

  const [syncConfirm, setSyncConfirm] = useState<{
    open: boolean;
    objectId: string;
    objectName: string;
    itemType: 'object' | 'folder';
    match: MatchedDbEntry | null;
    isLoading: boolean;
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

  type CurrentDataType = typeof syncConfirm.currentData;

  // ── Full scan preview ────────────────────────────────────────────
  const handleSync = useCallback(async () => {
    if (!activeGame || isSyncing) return;
    setIsSyncing(true);
    try {
      const previewItems = await scanService.scanPreview(
        activeGame.id,
        activeGame.game_type,
        activeGame.mod_path,
      );
      const dbJson = await scanService.getMasterDb(activeGame.game_type);
      const masterEntries = parseMasterDb(dbJson);

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

  // ── Commit scan results ──────────────────────────────────────────
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
          toast.error(`Destination exists: ${dest}. Please rename the folder or skip it.`);
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

  // ── Single-object DB sync ────────────────────────────────────────
  const handleSyncWithDb = useCallback(
    async (id: string, name: string) => {
      if (!activeGame) return;
      const folderMatch = folders.find((f) => f.path === id);
      const isFolder = !!folderMatch;
      const itemType = isFolder ? 'folder' : 'object';

      let currentData: CurrentDataType = null;
      if (isFolder && folderMatch) {
        currentData = {
          name: folderMatch.name,
          object_type: folderMatch.category ?? '',
          metadata: folderMatch.metadata ?? null,
          thumbnail_path: folderMatch.thumbnail_path,
        };
      } else {
        const obj = objects.find((o) => o.id === id);
        if (obj) {
          currentData = {
            name: obj.name,
            object_type: obj.object_type,
            metadata: null,
            thumbnail_path: obj.thumbnail_path,
          };
        }
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

  const handleApplySyncMatch = useCallback(
    async (match: MatchedDbEntry) => {
      const { objectId, itemType } = syncConfirm;
      if (!objectId) return;
      try {
        if (itemType === 'object') {
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
          let currentPath = objectId;
          const folder = folders.find((f) => f.path === objectId);
          if (folder && match.name !== folder.name) {
            const result = await invoke<{ new_path: string }>('rename_mod_folder', {
              folderPath: objectId,
              newName: match.name,
            });
            currentPath = result.new_path;
          }
          if (activeGame && match.object_type) {
            await invoke('set_mod_category', {
              gameId: activeGame.id,
              folderPath: currentPath,
              category: match.object_type,
            });
          }
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

  return {
    isSyncing,
    setIsSyncing,
    scanReview,
    setScanReview,
    syncConfirm,
    setSyncConfirm,
    handleSync,
    handleCommitScan,
    handleCloseScanReview,
    handleSyncWithDb,
    handleApplySyncMatch,
  };
}
