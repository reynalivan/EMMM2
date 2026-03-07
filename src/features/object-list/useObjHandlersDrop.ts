/**
 * useObjHandlersDrop — DnD drop zone handlers for ObjectList.
 * Extracted from useObjectListHandlers for SRP.
 */

import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { exists, mkdir } from '@tauri-apps/plugin-fs';
import { useActiveGame } from '../../hooks/useActiveGame';
import { scanService } from '../../lib/services/scanService';
import { toast } from '../../stores/useToastStore';
import { parseMasterDb, executeImportAndInvalidate } from './objHandlersHelpers';
import { classifyDroppedPaths } from './dropUtils';
import type { ObjectSummary } from '../../types/object';
import type { ScanPreviewItem } from '../../lib/services/scanService';
import type { MasterDbEntry } from './ScanReviewModal';

interface DropDeps {
  objects: ObjectSummary[];
  handleArchivesInteractively: (
    archivePaths: string[],
    context: {
      type: 'item' | 'auto-organize' | 'new-object';
      pathsToIngest: string[];
      targetFolder?: string;
      targetObjectId?: string;
      baseFolderPaths?: string[];
      baseLooseFiles?: string[];
    },
  ) => Promise<void>;
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

export function useObjHandlersDrop({
  objects,
  handleArchivesInteractively,
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

      await invoke('set_watcher_suppression_cmd', { suppressed: true });
      try {
        if (pathsToIngest.length === 0) {
          toast.info('No items to import.');
          return;
        }

        await executeImportAndInvalidate(pathsToIngest, objectFolderPath, queryClient, {
          isNewObject: false,
          objectName: obj.name,
        });
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

      if (classified.archives.length > 0) {
        handleArchivesInteractively(classified.archives, {
          type: 'auto-organize',
          baseFolderPaths: folderPaths,
          baseLooseFiles: looseFiles,
          pathsToIngest: [],
        });
        return;
      }

      await invoke('set_watcher_suppression_cmd', { suppressed: true });
      try {
        const tempPath = `${activeGame.mod_path}\\.emmm2_temp`;
        if (looseFiles.length > 0 && !(await exists(tempPath))) {
          await mkdir(tempPath, { recursive: true });
        }

        if (looseFiles.length > 0) {
          const ingestResult = await invoke<{
            moved: string[];
            skipped: string[];
            not_dirs: string[];
            sync: { new_mods: number; new_objects: number };
          }>('ingest_dropped_folders', {
            paths: looseFiles,
            modsPath: tempPath,
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
      } catch (e) {
        console.error('Auto organize failed:', e);
        toast.error(`Auto organize failed: ${e instanceof Error ? e.message : String(e)}`);
      } finally {
        setIsSyncing(false);
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
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
        if (!(await exists(objectFolderPath))) {
          await mkdir(objectFolderPath, { recursive: true });
        }
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

      await invoke('set_watcher_suppression_cmd', { suppressed: true });
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
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
      }
    },
    [activeGame, queryClient, handleArchivesInteractively],
  );

  /** No-op — ObjectList.tsx handles opening CreateObjectModal with pendingPaths */
  const handleDropNewObject = useCallback((_paths: string[]) => {
    // Intentionally empty — ObjectList.tsx handles opening CreateObjectModal
  }, []);

  return {
    handleDropOnItem,
    handleDropAutoOrganize,
    handleDropNewObject,
    handleDropOnNewObjectSubmit,
  };
}
