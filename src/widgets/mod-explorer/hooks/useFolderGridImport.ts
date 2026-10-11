import { useCallback } from 'react';
import { join } from '@tauri-apps/api/path';
import { useTranslation } from 'react-i18next';
import { useFileDrop, type DragPosition } from '../../../shared/lib/hooks/useFileDrop';
import { useDragAutoScroll } from '../../../shared/lib/hooks/useDragAutoScroll';
import { commands } from '../../../shared/api/tauri/bindings';
import { openImportBatchWizard } from '@/features/import-batches';
import { pathsEqual, pathStartsWith } from '@/shared/lib/pathKey';
import { formatAppError } from '@/shared/lib/appError';
import { toast } from '@/shared/ui/toast';

interface FolderGridImportOptions {
  parentRef: React.RefObject<HTMLDivElement | null>;
  dropTargetRef: React.RefObject<HTMLDivElement | null>;
  activeGameId: string | undefined;
  activeModPath: string | undefined;
  selectedObjectFolderPath: string | null;
  explorerSubPath: string | undefined;
}

export function useFolderGridImport({
  parentRef,
  dropTargetRef,
  activeGameId,
  activeModPath,
  selectedObjectFolderPath,
  explorerSubPath,
}: FolderGridImportOptions) {
  const { t } = useTranslation('common');
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
      let targetObjectId: string | null = null;
      for (const object of page.objects) {
        const absolutePath = await join(activeModPath, object.folder_path);
        if (
          pathsEqual(selectedObjectFolderPath, object.folder_path) ||
          pathsEqual(selectedObjectFolderPath, absolutePath)
        ) {
          targetObjectId = object.id;
          break;
        }
      }

      let targetSubpath: string | null = null;
      if (targetObjectId && selectedObjectFolderPath && explorerSubPath) {
        if (!pathStartsWith(selectedObjectFolderPath, explorerSubPath)) {
          throw new Error(t('errors.import_location_changed'));
        }
        const rootSegments = selectedObjectFolderPath.split(/[\\/]+/).filter(Boolean);
        targetSubpath =
          explorerSubPath
            .split(/[\\/]+/)
            .filter(Boolean)
            .slice(rootSegments.length)
            .join('/') || null;
      }

      openImportBatchWizard({
        kind: 'sources',
        gameId: activeGameId,
        flow: targetObjectId ? 'specific_import' : 'auto_import',
        targetMode: targetObjectId ? 'specific' : 'auto',
        targetObjectId,
        targetSubpath,
        paths,
      });
    },
    [activeGameId, activeModPath, explorerSubPath, selectedObjectFolderPath, t],
  );

  const handleNativeDrop = useCallback(
    async (paths: string[], position: DragPosition) => {
      const container = dropTargetRef.current;
      if (!container) return;
      const bounds = container.getBoundingClientRect();
      if (
        position.x < bounds.left ||
        position.x >= bounds.right ||
        position.y < bounds.top ||
        position.y >= bounds.bottom
      )
        return;
      try {
        await handleImportFiles(paths);
      } catch (error) {
        toast.error(formatAppError(error));
      }
    },
    [handleImportFiles, dropTargetRef],
  );
  const { isDragging, dragPosition } = useFileDrop({ onDrop: handleNativeDrop });
  useDragAutoScroll({ containerRef: parentRef, dragPosition });

  return { isDragging, handleImportFiles };
}
