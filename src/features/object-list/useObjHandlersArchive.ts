/**
 * useObjHandlersArchive — Archive modal state and extraction flow handlers.
 * Extracted from useObjectListHandlers for SRP.
 *
 * Receives scan state setters from orchestrator to avoid circular deps.
 */

import { useState, useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { basename } from '@tauri-apps/api/path';
import { exists, mkdir } from '@tauri-apps/plugin-fs';
import { useActiveGame } from '../../hooks/useActiveGame';
import { scanService, type ScanPreviewItem } from '../../lib/services/scanService';
import { toast } from '../../stores/useToastStore';
import { parseMasterDb, executeImportAndInvalidate } from './objHandlersHelpers';
import type { ObjectSummary } from '../../types/object';
import type { MasterDbEntry } from './ScanReviewModal';

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
}

export type PendingDropContext = {
  type: 'item' | 'auto-organize' | 'new-object';
  pathsToIngest: string[];
  targetFolder?: string;
  targetObjectId?: string;
  baseFolderPaths?: string[];
  baseLooseFiles?: string[];
};

export function useObjHandlersArchive({ objects, setScanReview, setIsSyncing }: ArchiveDeps) {
  const { activeGame } = useActiveGame();
  const queryClient = useQueryClient();

  const [archiveModal, setArchiveModal] = useState<{
    open: boolean;
    archives: import('../../types/scanner').ArchiveInfo[];
    isExtracting: boolean;
    error: string | null;
    pendingDropContext: PendingDropContext | null;
  }>({
    open: false,
    archives: [],
    isExtracting: false,
    error: null,
    pendingDropContext: null,
  });

  /** Launch ArchiveModal after analyzing archive files */
  const handleArchivesInteractively = useCallback(
    async (archivePaths: string[], context: PendingDropContext | null) => {
      const archiveInfos = await Promise.all(
        archivePaths.map(async (path) => {
          try {
            const analysis = await invoke<import('../../types/scanner').ArchiveAnalysis>(
              'analyze_archive_cmd',
              { archivePath: path },
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
    [],
  );

  /** Extract selected archives and resume the pending drop context flow */
  const handleArchiveExtractSubmit = useCallback(
    async (selectedPaths: string[], passwords: Record<string, string>, _overwrite?: boolean) => {
      const { pendingDropContext, archives } = archiveModal;
      if (!pendingDropContext || !activeGame) return;

      setArchiveModal((prev) => ({ ...prev, isExtracting: true, error: null }));
      await invoke('set_watcher_suppression_cmd', { suppressed: true });

      try {
        const extractedFolders: string[] = [];
        const extractTarget =
          pendingDropContext.type === 'item'
            ? pendingDropContext.targetFolder!
            : `${activeGame.mod_path}\\.emmm2_temp`;

        if (pendingDropContext.type === 'auto-organize' && !(await exists(extractTarget))) {
          await mkdir(extractTarget, { recursive: true });
        }

        const nonEncrypted = selectedPaths.filter(
          (p) => !archives.find((a) => a.path === p)?.is_encrypted,
        );
        const encrypted = selectedPaths.filter(
          (p) => !!archives.find((a) => a.path === p)?.is_encrypted,
        );

        const extractSingle = async (archivePath: string, pw?: string) => {
          const result = await scanService.extractArchive(archivePath, extractTarget, pw);
          if (result.success) {
            extractedFolders.push(...(result.dest_paths ?? []));
          } else {
            throw new Error(`Failed to extract ${await basename(archivePath)}: ${result.error}`);
          }
        };

        for (const archivePath of nonEncrypted) await extractSingle(archivePath);
        for (const archivePath of encrypted)
          await extractSingle(archivePath, passwords[archivePath]);

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

          await executeImportAndInvalidate(
            pathsToIngest,
            pendingDropContext.targetFolder!,
            queryClient,
            { isNewObject, objectName: obj?.name },
          );
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
        await invoke('set_watcher_suppression_cmd', { suppressed: false });
        if (pendingDropContext.type === 'auto-organize') setIsSyncing(false);
      }
    },
    [archiveModal, activeGame, objects, queryClient, setArchiveModal, setScanReview, setIsSyncing],
  );

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
  };
}
