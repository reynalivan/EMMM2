/**
 * useFolderGridImport — File import, drag-and-drop, and refresh.
 *
 * Extracted from useFolderGrid to keep the orchestrator under 350 lines.
 */

import { useCallback } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useImportMods, folderKeys } from '../../../hooks/useFolders';
import { useFileDrop } from '../../../hooks/useFileDrop';
import { useDragAutoScroll } from '../../../hooks/useDragAutoScroll';
import { join } from '@tauri-apps/api/path';
import { classifyDroppedPaths } from '../../object-list/dropUtils';

interface FolderGridImportOptions {
  parentRef: React.RefObject<HTMLDivElement | null>;
  activeModPath: string | undefined;
  explorerSubPath: string | undefined;
}

export function useFolderGridImport({
  parentRef,
  activeModPath,
  explorerSubPath,
}: FolderGridImportOptions) {
  const queryClient = useQueryClient();
  const importMods = useImportMods();

  const handleImportFiles = useCallback(
    async (paths: string[]) => {
      if (!activeModPath || paths.length === 0) return;

      const targetDir = explorerSubPath
        ? await join(activeModPath, explorerSubPath)
        : activeModPath;

      const classified = classifyDroppedPaths(paths);

      // Archives → dispatch to ObjectList's shared ArchiveModal for preview/extraction
      if (classified.archives.length > 0) {
        window.dispatchEvent(
          new CustomEvent('request-archive-import', {
            detail: {
              archives: classified.archives,
              nonArchivePaths: [
                ...classified.folders,
                ...classified.iniFiles,
                ...classified.images,
              ],
              targetDir,
            },
          }),
        );
      }

      // Non-archive paths → import directly via shared hook
      const nonArchivePaths = [...classified.folders, ...classified.iniFiles, ...classified.images];
      if (nonArchivePaths.length > 0) {
        importMods.mutate({ paths: nonArchivePaths, targetDir, strategy: 'Raw' });
      }
    },
    [activeModPath, explorerSubPath, importMods],
  );

  const { isDragging, dragPosition } = useFileDrop({ onDrop: handleImportFiles });

  useDragAutoScroll({
    containerRef: parentRef,
    dragPosition,
  });

  const handleRefresh = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: folderKeys.all });
  }, [queryClient]);

  return { isDragging, handleImportFiles, handleRefresh };
}
