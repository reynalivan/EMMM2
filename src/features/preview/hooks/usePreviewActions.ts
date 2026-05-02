import { useCallback, useRef, useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { useTranslation } from 'react-i18next';
import { commands } from '../../../lib/bindings';
import { toast } from '../../../stores/useToastStore';
import { publishWorkspaceIntent } from '../../workspace-runtime/workspaceIntentBus';
import type { WorkspaceExplorerNode } from '../../../types/workspace';

interface PreviewMutationLike<TInput> {
  isPending: boolean;
  mutateAsync: (input: TInput) => Promise<unknown>;
}

interface UsePreviewActionsOptions {
  activeGameId: string | null;
  activePath: string | null;
  selectedFolder: WorkspaceExplorerNode | null;
  images: string[];
  currentImagePath: string | null;
  setCurrentImageIndex: React.Dispatch<React.SetStateAction<number>>;
  savePreviewImage: PreviewMutationLike<{
    folderPath: string;
    objectName: string;
    imageData: number[];
  }>;
  removePreviewImage: PreviewMutationLike<{
    folderPath: string;
    imagePath: string;
  }>;
  clearPreviewImages: PreviewMutationLike<{
    folderPath: string;
  }>;
}

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function usePreviewActions({
  activeGameId,
  activePath,
  selectedFolder,
  images,
  currentImagePath,
  setCurrentImageIndex,
  savePreviewImage,
  removePreviewImage,
  clearPreviewImages,
}: UsePreviewActionsOptions) {
  const { t } = useTranslation(['preview', 'common']);
  const importInputRef = useRef<HTMLInputElement>(null);
  const [confirmRemoveOpen, setConfirmRemoveOpen] = useState(false);
  const [confirmClearOpen, setConfirmClearOpen] = useState(false);

  const saveImageBytes = useCallback(
    async (bytes: number[]) => {
      if (!activePath) {
        toast.warning(t('preview:empty.no_mod_selected'));
        return;
      }

      await savePreviewImage.mutateAsync({
        folderPath: activePath,
        objectName: selectedFolder?.name ?? 'mod',
        imageData: bytes,
      });
      setCurrentImageIndex(0);
    },
    [activePath, savePreviewImage, selectedFolder, setCurrentImageIndex, t],
  );

  const pasteThumbnailFromClipboard = useCallback(async () => {
    if (!activePath) {
      toast.warning(t('preview:empty.no_mod_selected'));
      return;
    }

    if (!navigator.clipboard?.read) {
      toast.error(t('common:errors.clipboard_not_supported'));
      return;
    }

    try {
      const items = await navigator.clipboard.read();
      let imageBlob: Blob | null = null;

      for (const item of items) {
        const imageType = item.types.find((type) => type.startsWith('image/'));
        if (imageType) {
          imageBlob = await item.getType(imageType);
          break;
        }
      }

      if (!imageBlob) {
        toast.warning(t('preview:gallery.no_image_in_clipboard'));
        return;
      }

      const bytes = Array.from(new Uint8Array(await imageBlob.arrayBuffer()));
      await saveImageBytes(bytes);
      toast.success(t('preview:gallery.thumbnail_pasted'));
    } catch (error) {
      toast.error(t('preview:gallery.paste_error', { error: toErrorMessage(error) }));
    }
  }, [activePath, saveImageBytes, t]);

  const triggerThumbnailImport = useCallback(() => {
    if (!activePath) {
      toast.warning(t('preview:empty.no_mod_selected'));
      return;
    }

    importInputRef.current?.click();
  }, [activePath, t]);

  const handleImportInputChange = useCallback(
    async (event: React.ChangeEvent<HTMLInputElement>) => {
      const file = event.target.files?.[0];
      event.currentTarget.value = '';

      if (!file) {
        return;
      }

      try {
        const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
        await saveImageBytes(bytes);
        toast.success(t('preview:gallery.menu.thumbnail_imported'));
      } catch (error) {
        toast.error(t('preview:gallery.import_error', { error: toErrorMessage(error) }));
      }
    },
    [saveImageBytes, t],
  );

  const requestRemoveCurrentImage = useCallback(() => {
    if (!currentImagePath) {
      toast.warning(t('preview:gallery.no_thumbnail_selected'));
      return;
    }

    setConfirmRemoveOpen(true);
  }, [currentImagePath, t]);

  const confirmRemoveCurrentImage = useCallback(async () => {
    setConfirmRemoveOpen(false);
    if (!activePath || !currentImagePath) {
      return;
    }

    try {
      await removePreviewImage.mutateAsync({
        folderPath: activePath,
        imagePath: currentImagePath,
      });
      setCurrentImageIndex((current) => Math.max(0, current - 1));
      toast.success(t('preview:gallery.thumbnail_removed'));
    } catch (error) {
      toast.error(t('preview:gallery.remove_error', { error: toErrorMessage(error) }));
    }
  }, [
    activePath,
    currentImagePath,
    removePreviewImage,
    setCurrentImageIndex,
    t,
  ]);

  const requestClearAllImages = useCallback(() => {
    if (images.length === 0) {
      toast.warning(t('preview:gallery.no_thumbnails_to_clear'));
      return;
    }

    setConfirmClearOpen(true);
  }, [images.length, t]);

  const confirmClearAllImages = useCallback(async () => {
    setConfirmClearOpen(false);
    if (!activePath) {
      return;
    }

    try {
      await clearPreviewImages.mutateAsync({ folderPath: activePath });
      setCurrentImageIndex(0);
      toast.success(t('preview:gallery.all_thumbnails_cleared'));
    } catch (error) {
      toast.error(t('preview:gallery.clear_all_error', { error: toErrorMessage(error) }));
    }
  }, [activePath, clearPreviewImages, setCurrentImageIndex, t]);

  const requestImportArchives = useCallback(async () => {
    const selected = await openDialog({
      multiple: true,
      filters: [{ name: 'Archives', extensions: ['zip', 'rar', '7z'] }],
    });
    if (!selected || !Array.isArray(selected) || selected.length === 0) {
      return;
    }

    publishWorkspaceIntent({
      type: 'autoOrganizePaths',
      paths: selected,
    });
  }, []);

  const requestImportFolders = useCallback(async () => {
    const selected = await openDialog({
      multiple: true,
      directory: true,
    });
    if (!selected || !Array.isArray(selected) || selected.length === 0) {
      return;
    }

    publishWorkspaceIntent({
      type: 'autoOrganizePaths',
      paths: selected,
    });
  }, []);

  const openCurrentLocation = useCallback(async () => {
    if (!activePath) {
      toast.warning(t('preview:empty.no_mod_selected'));
      return;
    }

    try {
      if (activeGameId) {
        await commands.openInExplorer({ gameId: activeGameId, path: activePath });
      }
    } catch (error) {
      toast.error(t('preview:errors.open_location_failed', { error: toErrorMessage(error) }));
    }
  }, [activeGameId, activePath, t]);

  return {
    importInputRef,
    confirmRemoveOpen,
    confirmClearOpen,
    setConfirmRemoveOpen,
    setConfirmClearOpen,
    pasteThumbnailFromClipboard,
    triggerThumbnailImport,
    handleImportInputChange,
    requestRemoveCurrentImage,
    confirmRemoveCurrentImage,
    requestClearAllImages,
    confirmClearAllImages,
    requestImportArchives,
    requestImportFolders,
    openCurrentLocation,
  };
}
