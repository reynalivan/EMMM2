/**
 * useObjHandlersDrop — DnD drop zone handlers for ObjectList.
 * Extracted from useObjectListHandlers for SRP.
 */

import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { commands, type IngestResult } from '../../lib/bindings';
import type { ScanPreviewItem } from '../../types/scanner';
import { useActiveGame } from '../../hooks/useActiveGame';
import { getGameTypeKey } from '../../types/game';
import { scanService } from '../../lib/services/scanService';
import { toast } from '../../stores/useToastStore';
import {
  executeImportAndInvalidate,
  parseMasterDb,
} from '../mod-runtime/operations/sharedOperations';
import { classifyDroppedPaths } from './dropUtils';
import type { ObjectSummary } from '../../types/object';
import type { MasterDbEntry } from './scanReviewHelpers';

interface PendingDropContext {
  type: 'item' | 'auto-organize' | 'new-object';
  pathsToIngest: string[];
  targetFolder?: string;
  targetObjectId?: string;
  baseFolderPaths?: string[];
  baseLooseFiles?: string[];
}

interface DropDeps {
  objects: ObjectSummary[];
  handleArchivesInteractively: (paths: string[], context: PendingDropContext) => Promise<void>;
  setMismatchConfirm: (paths: string[]) => void;
  setScanReview: React.Dispatch<
    React.SetStateAction<{
      open: boolean;
      items: ScanPreviewItem[];
      masterDbEntries: MasterDbEntry[];
      isCommitting: boolean;
    }>
  >;
  setIsSyncing: React.Dispatch<React.SetStateAction<boolean>>;
}

async function ensureDirectoryExists(path: string) {
  if (await commands.checkPathExistsCmd({ path })) {
    return;
  }

  await commands.ensureDir({ path });
}

export function useObjHandlersDrop({
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

      await commands.setWatcherSuppression({ suppressed: true });
      try {
        if (pathsToIngest.length === 0) {
          toast.info('No items to import.');
          return;
        }

        await executeImportAndInvalidate(pathsToIngest, objectFolderPath, queryClient, {
          isNewObject: false,
          objectName: obj.name,
        });

        if (classified.folders.length > 0) {
          try {
            let mismatches = 0;
            let firstMismatchMsg = '';
            const mismatchedPaths: string[] = [];
            for (const folder of classified.folders) {
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
                `${mismatches} of ${classified.folders.length} dropped folder(s) may not match ${obj.name}\n→ ${firstMismatchMsg}`,
                {
                  label: 'Fix',
                  onClick: () => setMismatchConfirm(mismatchedPaths),
                },
                9999999,
              );
            }
          } catch (err) {
            console.warn('Post-drop match check failed:', err);
          }
        }
      } catch (e) {
        console.error('Drop on item failed:', e);
        toast.error('Failed to import dropped items');
      } finally {
        await commands.setWatcherSuppression({ suppressed: false });
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

      await commands.setWatcherSuppression({ suppressed: true });
      try {
        const tempPath = `${activeGame.mod_path}\\.emmm_temp`;
        if (looseFiles.length > 0) {
          await ensureDirectoryExists(tempPath);
        }

        if (looseFiles.length > 0) {
          const ingestResult: IngestResult = await commands.ingestDroppedFolders({
            paths: looseFiles,
            modsPath: tempPath,
            gameId: activeGame.id,
            gameName: activeGame.name,
            gameType: getGameTypeKey(activeGame.game_type),
          });
          folderPaths.push(...ingestResult.moved);
        }

        setIsSyncing(true);
        const previewItemsRaw = await scanService.runDeepmatchPreview(
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
      } catch (e) {
        console.error('Auto organize failed:', e);
        toast.error(`Auto organize failed: ${e instanceof Error ? e.message : String(e)}`);
      } finally {
        setIsSyncing(false);
        await commands.setWatcherSuppression({ suppressed: false });
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

      await commands.setWatcherSuppression({ suppressed: true });
      try {
        if (pathsToIngest.length === 0) {
          toast.success(`Created ${objectName} successfully (no items imported).`);
          return;
        }

        await executeImportAndInvalidate(pathsToIngest, objectFolderPath, queryClient, {
          isNewObject: true,
          objectName,
        });
      } catch (e) {
        console.error('Drop on new object failed:', e);
        toast.error('Failed to import dropped items');
      } finally {
        await commands.setWatcherSuppression({ suppressed: false });
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
