/**
 * useArchiveImportFlow — archive modal state plus the extraction step of the
 * ObjectList import flow.
 *
 * A drop that contains archives parks its context here; once extraction
 * finishes (or is skipped) the parked drop resumes through the shared import
 * pipeline. Scan review state is owned by useScanReviewFlow and passed in.
 */

import { useCallback, useState, type Dispatch, type SetStateAction } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { basename } from '@tauri-apps/api/path';
import { commands } from '../../../lib/bindings';
import type { ArchiveInfo } from '../../../types/scanner';
import { buildArchiveInfo, buildExtractionSummary } from '../utils/archiveSummary';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { scanService } from '../../../lib/services/scanService';
import { toast } from '../../../stores/useToastStore';
import { executeImportAndInvalidate } from '../../mod-runtime/operations/sharedOperations';
import type { ObjectSummary } from '../../../types/object';
import { withWatcherSuppression } from '../../file-watcher/watcherSuppression';
import {
  buildScanReviewState,
  ensureDirectoryExists,
  ingestLooseFiles,
  tempStagingPath,
  warnOnFolderMismatch,
  type PendingDropContext,
  type ScanReviewState,
} from '../utils/importPipeline';

interface ArchiveDeps {
  objects: ObjectSummary[];
  setScanReview: Dispatch<SetStateAction<ScanReviewState>>;
  setIsSyncing: Dispatch<SetStateAction<boolean>>;
  setMismatchConfirm: (paths: string[]) => void;
}

interface ArchiveModalState {
  open: boolean;
  archives: ArchiveInfo[];
  isExtracting: boolean;
  error: string | null;
  passwordError: { path: string; message: string } | null;
  extractProgress: { current: number; total: number } | null;
  fileProgress: { fileName: string; fileIndex: number; totalFiles: number } | null;
  pendingDropContext: PendingDropContext | null;
}

const CLOSED_ARCHIVE_MODAL: ArchiveModalState = {
  open: false,
  archives: [],
  isExtracting: false,
  error: null,
  passwordError: null,
  extractProgress: null,
  fileProgress: null,
  pendingDropContext: null,
};

export function useArchiveImportFlow({
  objects,
  setScanReview,
  setIsSyncing,
  setMismatchConfirm,
}: ArchiveDeps) {
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();

  const [archiveModal, setArchiveModal] = useState<ArchiveModalState>(CLOSED_ARCHIVE_MODAL);

  /** Launch ArchiveModal after analyzing archive files */
  const handleArchivesInteractively = useCallback(
    async (archivePaths: string[], context: PendingDropContext | null) => {
      const archives = await Promise.all(
        archivePaths.map(async (path) => {
          const name = await basename(path);
          try {
            const analysis = await commands.analyzeArchive({ archivePath: path });
            return buildArchiveInfo(path, name, analysis);
          } catch (e) {
            console.error(`Failed to analyze archive ${path}:`, e);
            return buildArchiveInfo(path, name, null);
          }
        }),
      );

      setArchiveModal({
        ...CLOSED_ARCHIVE_MODAL,
        open: true,
        archives,
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
      try {
        await withWatcherSuppression({ releaseDelayMs: null }, async () => {
          const isAutoOrganize = pendingDropContext.type === 'auto-organize';
          const extractTarget = isAutoOrganize
            ? tempStagingPath(activeGame.mod_path)
            : activeGame.mod_path;

          if (isAutoOrganize) {
            await ensureDirectoryExists(extractTarget);
          }

          setArchiveModal((prev) => ({
            ...prev,
            extractProgress: { current: 0, total: selectedPaths.length },
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
          if (!batchResult.aborted) {
            const summary = buildExtractionSummary(batchResult.results);
            if (summary) toast.warning(summary);
          }

          setArchiveModal((prev) => ({
            ...prev,
            open: false,
            isExtracting: false,
            extractProgress: null,
            fileProgress: null,
          }));

          // Resume flow depending on context
          if (isAutoOrganize) {
            const folderPaths = [
              ...(pendingDropContext.baseFolderPaths || []),
              ...extractedFolders,
            ];
            folderPaths.push(
              ...(await ingestLooseFiles(
                activeGame,
                pendingDropContext.baseLooseFiles || [],
                extractTarget,
              )),
            );

            setIsSyncing(true);
            setScanReview(await buildScanReviewState(activeGame, folderPaths));
            return;
          }

          const obj = objects.find((o) => o.id === pendingDropContext.targetObjectId);
          const pathsToIngest = [...pendingDropContext.pathsToIngest, ...extractedFolders];
          if (pathsToIngest.length === 0) {
            toast.info('No items to import.');
            return;
          }

          await executeImportAndInvalidate(
            pathsToIngest,
            pendingDropContext.targetFolder!,
            queryClient,
            { isNewObject: pendingDropContext.type === 'new-object', objectName: obj?.name },
          );

          // Post-extraction match check for archives dropped on a specific object
          if (pendingDropContext.type === 'item' && obj?.name && extractedFolders.length > 0) {
            await warnOnFolderMismatch({
              activeGame,
              folders: extractedFolders,
              objectName: obj.name,
              noun: 'archive(s)',
              logLabel: 'Post-extraction match check failed:',
              onFix: setMismatchConfirm,
            });
          }
        });
      } catch (e: unknown) {
        console.error('Extraction flow failed:', e);
        setArchiveModal((prev) => ({
          ...prev,
          isExtracting: false,
          error: e instanceof Error ? e.message : String(e),
        }));
      } finally {
        if (pendingDropContext.type === 'auto-organize') setIsSyncing(false);
      }
    },
    [
      archiveModal,
      activeGame,
      objects,
      queryClient,
      setScanReview,
      setIsSyncing,
      setMismatchConfirm,
    ],
  );

  const handleStopExtraction = useCallback(async () => {
    try {
      await commands.abortExtraction();
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
        await withWatcherSuppression({ releaseDelayMs: null }, async () => {
          await executeImportAndInvalidate(
            pendingDropContext.pathsToIngest,
            pendingDropContext.targetFolder!,
            queryClient,
            { isNewObject, objectName: obj?.name },
          );
        });
      } catch (e) {
        console.error('Drop on item failed after skipping archives:', e);
        toast.error('Failed to import items');
      }
      return;
    }

    if (pendingDropContext.type !== 'auto-organize') return;

    try {
      const folderPaths = [...(pendingDropContext.baseFolderPaths || [])];
      const looseFiles = pendingDropContext.baseLooseFiles || [];
      if (folderPaths.length === 0 && looseFiles.length === 0) {
        toast.info('No items to auto-organize.');
        return;
      }

      setIsSyncing(true);
      await withWatcherSuppression({ releaseDelayMs: null }, async () => {
        folderPaths.push(
          ...(await ingestLooseFiles(activeGame, looseFiles, tempStagingPath(activeGame.mod_path))),
        );
        setScanReview(await buildScanReviewState(activeGame, folderPaths));
      });
    } catch (e: unknown) {
      console.error('Auto-organize failed post-skip:', e);
      toast.error(`Auto organize failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setIsSyncing(false);
    }
  }, [archiveModal, activeGame, objects, queryClient, setScanReview, setIsSyncing]);

  return {
    archiveModal,
    handleArchivesInteractively,
    handleArchiveExtractSubmit,
    handleArchiveExtractSkip,
    handleStopExtraction,
  };
}
