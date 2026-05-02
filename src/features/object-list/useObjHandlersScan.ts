import { useState, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { commands } from '../../lib/bindings';
import type { ConfirmedScanItem, ScanPreviewItem } from '../../types/scanner';
import { scanService } from '../../lib/services/scanService';
import { toast } from '../../stores/useToastStore';
import { parseMasterDb } from '../mod-runtime/operations/sharedOperations';
import { useTranslation } from 'react-i18next';
import { useActiveGame } from '../../hooks/useActiveGame';
import type { MasterDbEntry } from './scanReviewHelpers';
import { applyDiskReconcileResult } from '../file-watcher/hooks';
import { publishRuntimeDescriptor } from '../runtime-sync/queryRefresh';
import { buildRuntimeMutationDescriptor } from '../workspace-runtime/optimistic/descriptorBuilders';

export function useObjHandlersScan() {
  const { t } = useTranslation(['objects', 'common', 'scanner']);
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();

  // ── State ────────────────────────────────────────────────────────
  const [isSyncing, setIsSyncing] = useState(false);
  const isSyncingRef = useRef(false);

  const [scanReview, setScanReview] = useState<{
    open: boolean;
    items: ScanPreviewItem[];
    masterDbEntries: MasterDbEntry[];
    isCommitting: boolean;
  }>({ open: false, items: [], masterDbEntries: [], isCommitting: false });

  // ── Full scan preview ────────────────────────────────────────────
  const handleSync = useCallback(async () => {
    if (!activeGame || isSyncingRef.current) return;
    isSyncingRef.current = true;
    setIsSyncing(true);
    try {
      const previewItems = await scanService.runDeepmatchPreview(
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
      // Disk Reconcile only. This path repairs disk truth and must not run Deep Match Scanner.
      const result = await commands.reconcileDiskState({
        gameId: activeGame.id,
        reason: 'ManualRepair',
        forceFull: true,
      });
      applyDiskReconcileResult(result, queryClient, activeGame);
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
        await publishRuntimeDescriptor(
          queryClient,
          buildRuntimeMutationDescriptor('scannerWorkspaceState'),
          'active',
        );
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

  return {
    isSyncing,
    setIsSyncing,
    scanReview,
    setScanReview,
    handleSync,
    handleBackgroundSync,
    handleCommitScan,
    handleCloseScanReview,
  };
}
