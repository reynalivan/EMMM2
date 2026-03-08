/**
 * useObjHandlersArchive — Archive modal state and extraction flow handlers.
 * Extracted from useObjectListHandlers for SRP.
 *
 * Receives scan state setters from orchestrator to avoid circular deps.
 */

import { useCallback, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { basename } from '@tauri-apps/api/path';
import { exists, mkdir } from '@tauri-apps/plugin-fs';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useAppStore } from '../../stores/useAppStore';
import { scanService, type ScanPreviewItem } from '../../lib/services/scanService';
import { toast } from '../../stores/useToastStore';
import { parseMasterDb, executeImportAndInvalidate } from './objHandlersHelpers';
import type { ObjectSummary } from '../../types/object';
import type { MasterDbEntry } from './scanReviewHelpers';
import type { ArchiveAnalysis, ArchiveInfo } from '../../types/scanner';

interface ArchiveDeps {
  objects: ObjectSummary[];
  setScanReview: React.Dispatch<
    React.SetStateAction<{
      open: boolean;
      items: ScanPreviewItem[];
      masterDbEntries: MasterDbEntry[];
      isCommitting: boolean;
    }>
  >;
  setIsSyncing: React.Dispatch<React.SetStateAction<boolean>>;
  setMismatchConfirm: (paths: string[]) => void;
}

export type PendingDropContext = {
  type: 'item' | 'auto-organize' | 'new-object';
  pathsToIngest: string[];
  targetFolder?: string;
  targetObjectId?: string;
  baseFolderPaths?: string[];
  baseLooseFiles?: string[];
};

export function useObjHandlersArchive({
  objects,
  setScanReview,
  setIsSyncing,
  setMismatchConfirm,
}: ArchiveDeps) {
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();

  const [archiveModal, setArchiveModal] = useState<{
    open: boolean;
    archives: ArchiveInfo[];
    isExtracting: boolean;
    error: string | null;
    passwordError: { path: string; message: string } | null;
    extractProgress: { current: number; total: number } | null;
    fileProgress: { fileName: string; fileIndex: number; totalFiles: number } | null;
    pendingDropContext: PendingDropContext | null;
  }>({
    open: false,
    archives: [],
    isExtracting: false,
    error: null,
    passwordError: null,
    extractProgress: null,
    fileProgress: null,
    pendingDropContext: null,
  });

  /** Launch ArchiveModal after analyzing archive files */
  const handleArchivesInteractively = useCallback(
    async (archivePaths: string[], context: PendingDropContext | null) => {
      const archiveInfos = await Promise.all(
        archivePaths.map(async (path) => {
          try {
            const analysis = await invoke<ArchiveAnalysis>('analyze_archive_cmd', {
              archivePath: path,
            });
            const name = await basename(path);
            return {
              path,
              name,
              extension: name.split('.').pop() || '',
              size_bytes: analysis.file_size_bytes ?? 0,
              has_ini: analysis.has_ini,
              file_count: analysis.file_count ?? 1,
              is_encrypted: analysis.is_encrypted || false,
              contains_nested_archives: analysis.contains_nested_archives || false,
              entries: analysis.entries,
            } satisfies ArchiveInfo;
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
              contains_nested_archives: false,
            } as import('../../types/scanner').ArchiveInfo;
          }
        }),
      );
      setArchiveModal({
        open: true,
        archives: archiveInfos,
        isExtracting: false,
        error: null,
        passwordError: null,
        extractProgress: null,
        fileProgress: null,
        pendingDropContext: context,
      });
    },
    [],
  );

  /** Extract selected archives and resume the pending drop context flow */
  const handleArchiveExtractSubmit = useCallback(
    async (
      selectedPaths: string[],
      passwords: Record<string, string>,
      options?: {
        autoRename?: boolean;
        disableByDefault?: boolean;
        folderNames?: Record<string, string>;
        unpackNested?: boolean;
      },
    ) => {
      const { pendingDropContext, archives } = archiveModal;
      if (!pendingDropContext || !activeGame) return;

      setArchiveModal((prev) => ({
        ...prev,
        isExtracting: true,
        error: null,
        extractProgress: { current: 0, total: 0 },
      }));
      await invoke('set_watcher_suppression_cmd', { suppressed: true });

      try {
        const extractTarget =
          pendingDropContext.type === 'auto-organize'
            ? `${activeGame.mod_path}\\.emmm2_temp`
            : activeGame.mod_path;

        if (pendingDropContext.type === 'auto-organize' && !(await exists(extractTarget))) {
          await mkdir(extractTarget, { recursive: true });
        }

        const totalArchives = selectedPaths.length;
        setArchiveModal((prev) => ({
          ...prev,
          extractProgress: { current: 0, total: totalArchives },
        }));

        // A1: Use shared batch extraction utility
        const batchResult = await scanService.extractArchiveBatch(
          selectedPaths,
          archives,
          extractTarget,
          passwords,
          options,
          (current, total) => {
            setArchiveModal((prev) => ({
              ...prev,
              extractProgress: { current, total },
              fileProgress: null, // reset file progress between archives
            }));
          },
          (event) => {
            if (event.event === 'fileProgress') {
              setArchiveModal((prev) => ({
                ...prev,
                fileProgress: event.data,
              }));
            }
          },
        );

        const extractedFolders = batchResult.extractedPaths;
        const isAbortedLocally = batchResult.aborted;

        if (batchResult.isPasswordError && batchResult.failedPath) {
          // #5: Password error → keep modal open for retry
          setArchiveModal((prev) => ({
            ...prev,
            isExtracting: false,
            passwordError: { path: batchResult.failedPath!, message: batchResult.error! },
          }));
          return;
        }

        // #2: Queue summary — show toast for partial failures
        const done = batchResult.results.filter((r) => r.status === 'done').length;
        const failed = batchResult.results.filter((r) => r.status === 'failed').length;
        const skipped = batchResult.results.filter((r) => r.status === 'skipped').length;

        if (failed > 0 && !isAbortedLocally) {
          const failedNames = batchResult.results
            .filter((r) => r.status === 'failed')
            .map((r) => r.path.split('\\').pop() || r.path)
            .join(', ');
          const parts: string[] = [];
          if (done > 0) parts.push(`${done} extracted`);
          parts.push(`${failed} failed`);
          if (skipped > 0) parts.push(`${skipped} skipped`);
          toast.warning(`${parts.join(', ')}\nFailed: ${failedNames}`);
        }

        setArchiveModal((prev) => ({
          ...prev,
          open: false,
          isExtracting: false,
          extractProgress: null,
          fileProgress: null,
        }));

        // Resume flow depending on context
        if (pendingDropContext.type === 'item' || pendingDropContext.type === 'new-object') {
          const obj = objects.find((o) => o.id === pendingDropContext.targetObjectId);
          const pathsToIngest = [...pendingDropContext.pathsToIngest, ...extractedFolders];
          const isNewObject = pendingDropContext.type === 'new-object';

          if (pathsToIngest.length === 0) {
            toast.info('No items to import.');
            return;
          }

          await executeImportAndInvalidate(
            pathsToIngest,
            pendingDropContext.targetFolder!,
            queryClient,
            { isNewObject, objectName: obj?.name },
          );

          // Post-extraction match check for archives dropped on a specific object
          if (pendingDropContext.type === 'item' && obj?.name && extractedFolders.length > 0) {
            try {
              let mismatches = 0;
              let firstMismatchMsg = '';
              const mismatchedPaths: string[] = [];
              for (const folder of extractedFolders) {
                const check = await scanService.matchCheckFolder(
                  folder,
                  obj.name,
                  activeGame.game_type,
                );
                if (!check.isMatch) {
                  mismatches++;
                  mismatchedPaths.push(folder);
                  if (!firstMismatchMsg) {
                    firstMismatchMsg = `${folder.split('\\').pop()}: Best match is ${check.matchedName || 'Unknown'} (${check.matchScorePct}%)`;
                  }
                }
              }
              if (mismatches > 0) {
                toast.withAction(
                  'warning',
                  `${mismatches} of ${extractedFolders.length} archive(s) may not match ${obj.name}\n→ ${firstMismatchMsg}`,
                  {
                    label: 'Fix',
                    onClick: () => setMismatchConfirm(mismatchedPaths),
                  },
                  9999999,
                );
              }
            } catch (err) {
              console.warn('Post-extraction match check failed:', err);
            }
          }
        } else if (pendingDropContext.type === 'auto-organize') {
          const folderPaths = [...(pendingDropContext.baseFolderPaths || []), ...extractedFolders];
          const looseFiles = pendingDropContext.baseLooseFiles || [];

          if (looseFiles.length > 0) {
            const ingestResult = await invoke<{ moved: string[] }>('ingest_dropped_folders', {
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
            moveFromTemp: folderPaths.includes(item.folderPath),
          }));
          const dbJson = await scanService.getMasterDb(activeGame.game_type);
          const masterEntries = parseMasterDb(dbJson);

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
        // W2: Scale cooldown with archive count to avoid false watcher events
        const archiveCount = archiveModal.archives.length;
        const cooldown = Math.min(1000 + archiveCount * 500, 5000);
        useAppStore.getState().setWatcherCooldown(Date.now() + cooldown);
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
        if (pendingDropContext.type === 'auto-organize') setIsSyncing(false);
      }
    },
    [
      archiveModal,
      activeGame,
      objects,
      queryClient,
      setArchiveModal,
      setScanReview,
      setIsSyncing,
      setMismatchConfirm,
    ],
  );

  const handleStopExtraction = useCallback(async () => {
    try {
      await invoke('abort_extraction_cmd');
    } catch (e) {
      console.error('Failed to abort extraction:', e);
    }
  }, []);

  /** Skip extraction — proceed with non-archive items */
  const handleArchiveExtractSkip = useCallback(async () => {
    const { pendingDropContext } = archiveModal;
    setArchiveModal((prev) => ({ ...prev, open: false }));

    if (!pendingDropContext || !activeGame) return;

    if (pendingDropContext.type === 'item' || pendingDropContext.type === 'new-object') {
      const obj = objects.find((o) => o.id === pendingDropContext.targetObjectId);
      const isNewObject = pendingDropContext.type === 'new-object';
      if (pendingDropContext.pathsToIngest.length === 0) {
        toast.info('No items to import.');
        return;
      }

      try {
        await invoke('set_watcher_suppression_cmd', { suppressed: true });
        await executeImportAndInvalidate(
          pendingDropContext.pathsToIngest,
          pendingDropContext.targetFolder!,
          queryClient,
          { isNewObject, objectName: obj?.name },
        );
      } catch (e) {
        console.error('Drop on item failed after skipping archives:', e);
        toast.error('Failed to import items');
      } finally {
        useAppStore.getState().setWatcherCooldown(Date.now() + 1000);
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
      }
    } else if (pendingDropContext.type === 'auto-organize') {
      try {
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
          if (!(await exists(extractTarget))) await mkdir(extractTarget, { recursive: true });
          const ingestResult = await invoke<{ moved: string[] }>('ingest_dropped_folders', {
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
        const masterEntries = parseMasterDb(dbJson);

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
        useAppStore.getState().setWatcherCooldown(Date.now() + 1000);
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
      }
    }
  }, [
    archiveModal,
    activeGame,
    objects,
    queryClient,
    setArchiveModal,
    setScanReview,
    setIsSyncing,
  ]);

  return {
    archiveModal,
    handleArchivesInteractively,
    handleArchiveExtractSubmit,
    handleArchiveExtractSkip,
    handleStopExtraction,
  };
}
