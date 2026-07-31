/**
 * useDropImportFlow — entry point of the ObjectList import flow.
 *
 * Classifies dropped paths and either hands archives to the archive modal or
 * runs the shared import pipeline directly.
 */

import { useCallback, type Dispatch, type SetStateAction } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { toast } from '../../../stores/useToastStore';
import { executeImportAndInvalidate } from '../../mod-runtime/operations/sharedOperations';
import { classifyDroppedPaths } from '../utils/dropUtils';
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

interface DropDeps {
  objects: ObjectSummary[];
  handleArchivesInteractively: (paths: string[], context: PendingDropContext) => Promise<void>;
  setMismatchConfirm: (paths: string[]) => void;
  setScanReview: Dispatch<SetStateAction<ScanReviewState>>;
  setIsSyncing: Dispatch<SetStateAction<boolean>>;
}

export function useDropImportFlow({
  objects,
  handleArchivesInteractively,
  setMismatchConfirm,
  setScanReview,
  setIsSyncing,
}: DropDeps) {
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();

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

      const pathsToIngest = [...classified.folders, ...classified.iniFiles, ...classified.images];

      if (classified.archives.length > 0) {
        handleArchivesInteractively(classified.archives, {
          type: 'item',
          pathsToIngest,
          targetFolder: objectFolderPath,
          targetObjectId: objectId,
        });
        return;
      }

      try {
        await withWatcherSuppression({ releaseDelayMs: null }, async () => {
          if (pathsToIngest.length === 0) {
            toast.info('No items to import.');
            return;
          }

          await executeImportAndInvalidate(pathsToIngest, objectFolderPath, queryClient, {
            isNewObject: false,
            objectName: obj.name,
          });

          if (classified.folders.length > 0) {
            await warnOnFolderMismatch({
              activeGame,
              folders: classified.folders,
              objectName: obj.name,
              noun: 'dropped folder(s)',
              logLabel: 'Post-drop match check failed:',
              onFix: setMismatchConfirm,
            });
          }
        });
      } catch (e) {
        console.error('Drop on item failed:', e);
        toast.error('Failed to import dropped items');
      }
    },
    [activeGame, objects, queryClient, handleArchivesInteractively, setMismatchConfirm],
  );

  /** Drop on Auto Organize zone — extract archives → scan → open review */
  const handleDropAutoOrganize = useCallback(
    async (paths: string[]) => {
      if (!activeGame) return;

      const classified = classifyDroppedPaths(paths);
      toast.info('Preparing Auto Organize...');

      const folderPaths: string[] = [...classified.folders];
      const looseFiles = [...classified.iniFiles, ...classified.images];

      if (classified.archives.length > 0) {
        handleArchivesInteractively(classified.archives, {
          type: 'auto-organize',
          baseFolderPaths: folderPaths,
          baseLooseFiles: looseFiles,
          pathsToIngest: [],
        });
        return;
      }

      try {
        await withWatcherSuppression({ releaseDelayMs: null }, async () => {
          folderPaths.push(
            ...(await ingestLooseFiles(looseFiles, tempStagingPath(activeGame.mod_path))),
          );

          setIsSyncing(true);
          setScanReview(await buildScanReviewState(activeGame, folderPaths));
        });
      } catch (e) {
        console.error('Auto organize failed:', e);
        toast.error(`Auto organize failed: ${e instanceof Error ? e.message : String(e)}`);
      } finally {
        setIsSyncing(false);
      }
    },
    [activeGame, handleArchivesInteractively, setScanReview, setIsSyncing],
  );

  /** Called by CreateObjectModal after creating a DB shell to physically ingest files */
  const handleDropOnNewObjectSubmit = useCallback(
    async (newObjectId: string, objectName: string, paths: string[]) => {
      if (!activeGame || paths.length === 0) return;

      toast.info(`Importing item(s) to ${objectName}...`);
      const classified = classifyDroppedPaths(paths);
      const objectFolderPath = `${activeGame.mod_path}\\${objectName}`;

      const pathsToIngest = [...classified.folders, ...classified.iniFiles, ...classified.images];

      try {
        await ensureDirectoryExists(objectFolderPath);
      } catch (e) {
        console.error('Failed to create object directory on disk:', e);
        toast.error('Failed to create object directory on disk.');
        return;
      }

      if (classified.archives.length > 0) {
        handleArchivesInteractively(classified.archives, {
          type: 'new-object',
          pathsToIngest,
          targetFolder: objectFolderPath,
          targetObjectId: newObjectId,
        });
        return;
      }

      try {
        await withWatcherSuppression({ releaseDelayMs: null }, async () => {
          if (pathsToIngest.length === 0) {
            toast.success(`Created ${objectName} successfully (no items imported).`);
            return;
          }

          await executeImportAndInvalidate(pathsToIngest, objectFolderPath, queryClient, {
            isNewObject: true,
            objectName,
          });
        });
      } catch (e) {
        console.error('Drop on new object failed:', e);
        toast.error('Failed to import dropped items');
      }
    },
    [activeGame, queryClient, handleArchivesInteractively],
  );

  return {
    handleDropOnItem,
    handleDropAutoOrganize,
    handleDropOnNewObjectSubmit,
  };
}
