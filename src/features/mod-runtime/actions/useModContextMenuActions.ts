import { useCallback } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../lib/bindings';
import { usePasteThumbnail, useUpdateModThumbnail } from '../../../hooks/useFolderMutations';
import { useActiveGame } from '../../../hooks/useActiveGame';
import { toast } from '../../../stores/useToastStore';
import type { WorkspaceExplorerNode } from '../../../types/workspace';

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function useModContextMenuActions(folder: WorkspaceExplorerNode) {
  const { t } = useTranslation(['grid', 'preview', 'common']);
  const { activeGame } = useActiveGame();
  const pasteThumbnail = usePasteThumbnail();
  const updateModThumbnail = useUpdateModThumbnail();

  const openExplorer = useCallback(async () => {
    if (!activeGame?.id) {
      return;
    }

    try {
      await commands.openInExplorer({ gameId: activeGame.id, path: folder.path });
    } catch (error) {
      toast.error(t('preview:errors.open_location_failed', { error: toErrorMessage(error) }));
    }
  }, [activeGame, folder.path, t]);

  const pasteThumbnailFromClipboard = useCallback(async () => {
    if (!navigator.clipboard?.read) {
      toast.error(t('common:errors.clipboard_not_supported'));
      return;
    }

    try {
      const items = await navigator.clipboard.read();
      let imageBytes: number[] | null = null;

      for (const item of items) {
        const imageType = item.types.find((type) => type.startsWith('image/'));
        if (!imageType) {
          continue;
        }

        const blob = await item.getType(imageType);
        imageBytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
        break;
      }

      if (!imageBytes) {
        toast.warning(t('preview:gallery.no_image_in_clipboard'));
        return;
      }

      await pasteThumbnail.mutateAsync({
        folderPath: folder.path,
        imageData: imageBytes,
      });
      toast.success(t('preview:gallery.thumbnail_pasted'));
    } catch (error) {
      toast.error(t('preview:gallery.paste_error', { error: toErrorMessage(error) }));
    }
  }, [folder.path, pasteThumbnail, t]);

  const importThumbnail = useCallback(async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      });

      if (!selected || typeof selected !== 'string') {
        return;
      }

      await updateModThumbnail.mutateAsync({
        folderPath: folder.path,
        sourcePath: selected,
      });
      toast.success(t('preview:gallery.menu.thumbnail_imported'));
    } catch (error) {
      toast.error(t('preview:gallery.import_error', { error: toErrorMessage(error) }));
    }
  }, [folder.path, t, updateModThumbnail]);

  return {
    openExplorer,
    pasteThumbnailFromClipboard,
    importThumbnail,
  };
}
