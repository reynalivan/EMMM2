import { useState, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { commands } from '../../lib/bindings';
import type { ScanPreviewItem, ConfirmedScanItem } from '../../types/scanner';

import { scanService } from '../../lib/services/scanService';
import { useUpdateObject } from '../../hooks/useObjects';
import { toast } from '../../stores/useToastStore';
import { corridorKeys } from '../collections/queryKeys';
import { parseMasterDb } from './objHandlersHelpers';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '../../hooks/useActiveGame';
import type { ObjectSummary } from '../../types/object';
import type { ModFolder } from '../../hooks/useFolders';
import type { MatchedDbEntry } from './SyncConfirmModal';
import type { MasterDbEntry } from './scanReviewHelpers';

interface ScanDeps {
  objects: ObjectSummary[];
  folders: ModFolder[];
}

export function useObjHandlersScan({ objects, folders }: ScanDeps) {
  const { t } = useTranslation(['objects', 'common', 'scanner']);
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();
  const updateObject = useUpdateObject();

  // ── State ────────────────────────────────────────────────────────
  const [isSyncing, setIsSyncing] = useState(false);
  const isSyncingRef = useRef(false);

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
    if (!activeGame || isSyncingRef.current) return;
    isSyncingRef.current = true;
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
      toast.error(t('scanner:scan_failed', { error: e instanceof Error ? e.message : String(e) }));
    } finally {
      isSyncingRef.current = false;
      setIsSyncing(false);
    }
  }, [activeGame, t]);

  // ── Background Indexing ──────────────────────────────────────────
  const handleBackgroundSync = useCallback(async () => {
    if (!activeGame || isSyncingRef.current) return;
    isSyncingRef.current = true;
    setIsSyncing(true);
    try {
      await scanService.quickImport(
        activeGame.id,
        activeGame.name,
        activeGame.game_type,
        activeGame.mod_path,
      );
      queryClient.invalidateQueries({ queryKey: ['objects'] });
      queryClient.invalidateQueries({ queryKey: ['mod-folders'] });
      queryClient.invalidateQueries({ queryKey: ['category-counts'] });
      queryClient.invalidateQueries({ queryKey: corridorKeys.all });
    } catch (e) {
      console.error('Background sync failed:', e);
    } finally {
      isSyncingRef.current = false;
      setIsSyncing(false);
    }
  }, [activeGame, queryClient]);

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
        queryClient.invalidateQueries({ queryKey: corridorKeys.all });
        toast.success(
          t('objects:sync.toast.complete', {
            scanned: result.total_scanned,
            newMods: result.new_mods,
            newObjects: result.new_objects,
          }),
        );
        setScanReview({ open: false, items: [], masterDbEntries: [], isCommitting: false });
      } catch (e) {
        console.error('Commit scan failed:', e);
        const errMsg = e instanceof Error ? e.message : String(e);
        if (errMsg.includes('DUPLICATE|')) {
          const dest = errMsg.split('DUPLICATE|')[1] || '';
          toast.error(t('objects:sync.toast.destination_exists', { dest }));
        } else {
          toast.error(t('objects:sync.toast.commit_failed', { error: errMsg }));
        }
        setScanReview((prev) => ({ ...prev, isCommitting: false }));
      }
    },
    [activeGame, queryClient, t],
  );

  const handleCloseScanReview = useCallback(async () => {
    if (scanReview.isCommitting) return;
    setScanReview({ open: false, items: [], masterDbEntries: [], isCommitting: false });
  }, [scanReview.isCommitting]);

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
        const match = await commands.matchObjectWithDb({
          gameType: activeGame.game_type,
          objectName: name,
        });

        setSyncConfirm((prev) => ({ ...prev, match: match as MatchedDbEntry, isLoading: false }));
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
            const result = await commands.renameModFolder({
              folderPath: objectId,
              newName: match.name,
              gameId: activeGame?.id ?? '',
            });
            currentPath = result.new_path;
          }
          if (activeGame && match.object_type) {
            await commands.setModCategory({
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
            await commands.updateModInfo({
              folderPath: currentPath,
              update: { metadata: metaStrings },
            });
          }
          if (match.thumbnail_path) {
            await commands.updateModThumbnail({
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
    handleBackgroundSync,
    handleCommitScan,
    handleCloseScanReview,
    handleSyncWithDb,
    handleApplySyncMatch,
  };
}
