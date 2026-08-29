import { useCallback } from 'react';
import { join } from '@tauri-apps/api/path';
import { useFileDrop } from '../../../hooks/useFileDrop';
import { useDragAutoScroll } from '../../../hooks/useDragAutoScroll';
import { commands } from '../../../lib/bindings';
import { openImportBatchWizard } from '../../import-batches/launcher';

interface FolderGridImportOptions {
  parentRef: React.RefObject<HTMLDivElement | null>;
  activeGameId: string | undefined;
  activeModPath: string | undefined;
  selectedObjectFolderPath: string | null;
  explorerSubPath: string | undefined;
}

function pathKey(path: string): string {
  return path.replace(/\\/g, '/').replace(/\/+$/, '').toLocaleLowerCase();
}

export function useFolderGridImport({
  parentRef,
  activeGameId,
  activeModPath,
  selectedObjectFolderPath,
  explorerSubPath,
}: FolderGridImportOptions) {
  const handleImportFiles = useCallback(
    async (paths: string[]) => {
      if (!activeGameId || !activeModPath || paths.length === 0) return;

      const page = await commands.getObjectsCmd({
        game_id: activeGameId,
        search_query: null,
        object_type: null,
        meta_filters: null,
        sort_by: null,
        status_filter: null,
      });
      const selectedKey = selectedObjectFolderPath ? pathKey(selectedObjectFolderPath) : null;
      let targetObjectId: string | null = null;
      for (const object of page.objects) {
        const absolutePath = await join(activeModPath, object.folder_path);
        if (selectedKey === pathKey(absolutePath)) {
          targetObjectId = object.id;
          break;
        }
      }

      openImportBatchWizard({
        kind: 'sources',
        gameId: activeGameId,
        flow: targetObjectId ? 'specific_import' : 'auto_import',
        targetMode: targetObjectId ? 'specific' : 'auto',
        targetObjectId,
        targetSubpath: explorerSubPath ?? null,
        paths,
      });
    },
    [activeGameId, activeModPath, explorerSubPath, selectedObjectFolderPath],
  );

  const { isDragging, dragPosition } = useFileDrop({ onDrop: handleImportFiles });
  useDragAutoScroll({ containerRef: parentRef, dragPosition });

  return { isDragging, handleImportFiles };
}
