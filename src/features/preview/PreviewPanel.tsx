import { useCallback, useEffect, useRef, useState } from 'react';
import { ChevronRight, Info, X, FolderOpen, FileArchive, FolderPlus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { useAppStore } from '../../stores/useAppStore';
import { toast } from '../../stores/useToastStore';
import ConfirmDialog from '../../components/ui/ConfirmDialog';
import UnsavedIniChangesModal from './components/UnsavedIniChangesModal';
import GallerySection from './components/GallerySection';
import MetadataSection from './components/MetadataSection';
import IniEditorSection from './components/IniEditorSection';
import { useActiveGame } from '../../hooks/useActiveGame';
import { usePreviewPanelState } from './hooks/usePreviewPanelState';
import { usePreviewPanelActions } from './hooks/usePreviewPanelActions';
import PreviewPanelModals from './components/PreviewPanelModals';
import { commands } from '../../lib/bindings';
import PreviewPanelContextMenu from './components/PreviewPanelContextMenu';

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default function PreviewPanel() {
  const { t } = useTranslation(['preview', 'common']);
  const setMobilePane = useAppStore((state) => state.setMobilePane);
  const setSelectedObjectFolderPath = useAppStore((state) => state.setSelectedObjectFolderPath);

  const importInputRef = useRef<HTMLInputElement>(null);
  const [confirmRemoveOpen, setConfirmRemoveOpen] = useState(false);
  const [confirmClearOpen, setConfirmClearOpen] = useState(false);
  const { activeGame } = useActiveGame();

  const {
    activePath,
    selectedFolder,
    images,
    currentImageIndex,
    setCurrentImageIndex,
    titleDraft,
    authorDraft,
    versionDraft,
    descriptionDraft,
    setTitleDraft,
    setAuthorDraft,
    setVersionDraft,
    setDescriptionDraft,
    metadataDirty,
    keyBindSections,
    openSectionIds,
    draftByField,
    fieldErrors,
    conflictingKeys,
    hasUnsavedEditorChanges,
    changedIniFields,
    changedMetadataFields,
    savePreviewImage,
    removePreviewImage,
    clearPreviewImages,
    writeModIni,
    previewImagesQuery,
    toggleMod,
    showUnsavedModal,
    setShowUnsavedModal,
    setPendingTransition,
    pendingTransition,
    applyPendingTransition,
    saveMetadata,
    discardMetadata,
    saveEditor,
    discardEditor,
    requestToggleSection,
    updateEditorField,
  } = usePreviewPanelState();
  const actions = usePreviewPanelActions();

  const boundedImageIndex = Math.min(currentImageIndex, Math.max(images.length - 1, 0));
  const currentImagePath = images[boundedImageIndex] ?? null;

  const [isScrolled, setIsScrolled] = useState(false);
  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    setIsScrolled(e.currentTarget.scrollTop > 10);
  }, []);

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
        toast.warning(t('preview:gallery.menu.no_image_in_clipboard'));
        return;
      }

      const bytes = Array.from(new Uint8Array(await imageBlob.arrayBuffer()));
      await savePreviewImage.mutateAsync({
        folderPath: activePath,
        objectName: selectedFolder?.name ?? 'mod',
        imageData: bytes,
      });
      await previewImagesQuery.refetch();
      setCurrentImageIndex(0);
      toast.success(t('preview:gallery.menu.thumbnail_pasted'));
    } catch (error) {
      toast.error(t('preview:gallery.menu.paste_error', { error: toErrorMessage(error) }));
    }
  }, [activePath, savePreviewImage, selectedFolder, previewImagesQuery, setCurrentImageIndex, t]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const isPaste = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'v';
      if (!isPaste || !activePath) {
        return;
      }

      const target = event.target as HTMLElement | null;
      const editable =
        target?.tagName === 'INPUT' ||
        target?.tagName === 'TEXTAREA' ||
        target?.isContentEditable === true;
      if (editable) {
        return;
      }

      event.preventDefault();
      void pasteThumbnailFromClipboard();
    };

    window.addEventListener('keydown', onKeyDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
    };
  }, [activePath, pasteThumbnailFromClipboard]);

  if (!activePath) {
    return (
      <div className="mx-auto flex h-full w-full max-w-140 flex-col items-center justify-center p-6 text-center border-l border-base-content/5 bg-base-100/30 backdrop-blur-md">
        <div className="mb-6 text-base-content/50">
          <FolderOpen size={48} className="mx-auto mb-4 opacity-50" />
          <p className="text-xl font-bold text-base-content mb-2">
            {t('preview:empty.no_mod_selected')}
          </p>
          <p className="text-sm">{t('preview:empty.select_folder')}</p>
        </div>
        <div className="flex w-full max-w-xs flex-col gap-3">
          <button
            className="btn btn-outline btn-primary gap-2"
            onClick={async () => {
              const selected = await openDialog({
                multiple: true,
                filters: [{ name: 'Archives', extensions: ['zip', 'rar', '7z'] }],
              });
              if (selected && selected.length > 0) {
                window.dispatchEvent(
                  new CustomEvent('request-auto-organize-paths', { detail: selected }),
                );
              }
            }}
          >
            <FileArchive size={18} />
            {t('preview:actions.import_archives')}
          </button>
          <button
            className="btn btn-outline btn-primary gap-2"
            onClick={async () => {
              const selected = await openDialog({
                multiple: true,
                directory: true,
              });
              if (selected && selected.length > 0) {
                window.dispatchEvent(
                  new CustomEvent('request-auto-organize-paths', { detail: selected }),
                );
              }
            }}
          >
            <FolderPlus size={18} />
            {t('preview:actions.import_folders')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div
      className="mx-auto flex h-full w-full max-w-140 flex-col overflow-y-auto border-l border-base-content/5 bg-base-100/30 px-6 pb-6 pt-0 backdrop-blur-md"
      onScroll={handleScroll}
    >
      <input
        ref={importInputRef}
        type="file"
        accept="image/png,image/jpeg,image/webp,image/gif"
        className="hidden"
        onChange={async (event) => {
          const file = event.target.files?.[0];
          event.currentTarget.value = '';

          if (!file || !activePath) {
            return;
          }

          try {
            const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
            await savePreviewImage.mutateAsync({
              folderPath: activePath,
              objectName: selectedFolder?.name ?? 'mod',
              imageData: bytes,
            });
            await previewImagesQuery.refetch();
            setCurrentImageIndex(0);
            toast.success(t('preview:gallery.menu.thumbnail_imported'));
          } catch (error) {
            toast.error(t('preview:gallery.menu.import_error', { error: toErrorMessage(error) }));
          }
        }}
      />

      <ConfirmDialog
        open={confirmRemoveOpen}
        title={t('preview:gallery.menu.remove_current')}
        message={t('preview:gallery.remove_confirm_message')}
        confirmLabel={t('common:actions.remove')}
        cancelLabel={t('common:actions.cancel')}
        danger
        onCancel={() => setConfirmRemoveOpen(false)}
        onConfirm={async () => {
          setConfirmRemoveOpen(false);
          if (!activePath || !currentImagePath) {
            return;
          }
          try {
            await removePreviewImage.mutateAsync({
              folderPath: activePath,
              imagePath: currentImagePath,
            });
            await previewImagesQuery.refetch();
            setCurrentImageIndex((prev) => Math.max(0, prev - 1));
            toast.success(t('preview:gallery.menu.thumbnail_removed'));
          } catch (error) {
            toast.error(t('preview:gallery.menu.remove_error', { error: toErrorMessage(error) }));
          }
        }}
      />

      <ConfirmDialog
        open={confirmClearOpen}
        title={t('preview:gallery.menu.clear_all')}
        message={t('preview:gallery.clear_all_confirm_message')}
        confirmLabel={t('preview:gallery.menu.clear_all')}
        cancelLabel={t('common:actions.cancel')}
        danger
        onCancel={() => setConfirmClearOpen(false)}
        onConfirm={async () => {
          setConfirmClearOpen(false);
          if (!activePath) {
            return;
          }
          try {
            await clearPreviewImages.mutateAsync({ folderPath: activePath });
            await previewImagesQuery.refetch();
            setCurrentImageIndex(0);
            toast.success(t('preview:gallery.menu.all_thumbnails_cleared'));
          } catch (error) {
            toast.error(
              t('preview:gallery.menu.clear_all_error', { error: toErrorMessage(error) }),
            );
          }
        }}
      />

      <UnsavedIniChangesModal
        open={showUnsavedModal}
        isSaving={writeModIni.isPending}
        modName={titleDraft || selectedFolder?.name}
        categoryName={selectedFolder?.category ?? undefined}
        changedIniFields={changedIniFields}
        changedMetadataFields={changedMetadataFields}
        onCancel={() => {
          setShowUnsavedModal(false);
          setPendingTransition(null);
        }}
        onDiscard={() => {
          discardMetadata();
          discardEditor();
          applyPendingTransition(pendingTransition);
          setShowUnsavedModal(false);
          setPendingTransition(null);
        }}
        onSave={async () => {
          await saveMetadata();
          const editorSaved = await saveEditor();
          if (!editorSaved) return;
          applyPendingTransition(pendingTransition);
          setShowUnsavedModal(false);
          setPendingTransition(null);
        }}
      />

      <div
        className={`sticky top-0 z-20 -mx-6 mb-6 px-6 transition-all duration-200 flex flex-col justify-center border-b ${
          isScrolled
            ? 'pt-4 pb-2 bg-base-100/95 backdrop-blur-md border-base-content/10 shadow-sm'
            : 'pt-6 pb-2 bg-transparent border-transparent'
        }`}
      >
        <div className={`flex items-center justify-between transition-all duration-200`}>
          <div className="min-w-0 flex-1">
            <div className={`flex items-center gap-2 transition-all duration-200 mb-0`}>
              <button
                onClick={() => setMobilePane('grid')}
                aria-label={t('preview:actions.back_to_grid')}
                className={`btn btn-circle btn-ghost text-base-content/50 hover:text-base-content md:hidden transition-all duration-200 ${isScrolled ? 'btn-xs' : 'btn-sm'}`}
              >
                <ChevronRight className="rotate-180" size={isScrolled ? 14 : 16} />
              </button>
              <input
                type="text"
                className={`bg-transparent p-0 m-0 border-none outline-none focus:ring-1 focus:ring-primary focus:bg-base-200/50 rounded px-1 -ml-1 truncate tracking-tight text-base-content transition-all duration-200 origin-left hover:bg-base-content/5 ${
                  isScrolled ? 'text-sm font-semibold' : 'text-xl font-bold'
                }`}
                value={titleDraft || ''}
                placeholder={selectedFolder?.name || t('preview:empty.no_mod_selected')}
                onChange={(e) => setTitleDraft(e.target.value)}
                disabled={!activePath}
              />
            </div>
            <label
              className={`label cursor-pointer justify-start gap-2 p-0 opacity-80 hover:opacity-100 transition-all duration-200 ${isScrolled ? '-mt-0.5' : 'mt-1'}`}
            >
              <input
                type="checkbox"
                aria-label={t('preview:actions.toggle_enabled')}
                className={`toggle border-base-content/10 bg-base-300 checked:border-primary checked:bg-primary transition-all duration-200 ${
                  isScrolled ? 'toggle-xs' : 'toggle-sm'
                }`}
                checked={selectedFolder?.is_enabled ?? false}
                disabled={!selectedFolder || toggleMod.isPending}
                onChange={() => {
                  if (!selectedFolder || !activeGame?.id) return;
                  toggleMod.mutate({
                    path: selectedFolder.path,
                    enable: !selectedFolder.is_enabled,
                    gameId: activeGame.id,
                  });
                }}
              />
              <span
                className={`font-medium text-base-content/60 transition-all duration-200 ${isScrolled ? 'text-[10px]' : 'text-sm'}`}
              >
                {selectedFolder
                  ? selectedFolder.is_enabled
                    ? t('common:status.enabled')
                    : t('common:status.disabled')
                  : t('preview:empty.no_mod_selected')}
              </span>
            </label>
          </div>

          <div className="ml-2 flex items-center gap-1">
            {selectedFolder && (
              <PreviewPanelContextMenu
                folder={selectedFolder}
                onRename={() => actions.handleRenameRequest(selectedFolder)}
                onDelete={() => actions.handleDeleteRequest(selectedFolder)}
                onToggle={actions.handleToggleEnabled}
                onToggleFavorite={actions.handleToggleFavorite}
                onEnableOnlyThis={actions.handleEnableOnlyThis}
                onOpenMoveDialog={actions.openMoveDialog}
                onToggleSafe={actions.handleToggleSafeRequest}
              />
            )}
            <button
              onClick={() => setSelectedObjectFolderPath(null)}
              aria-label={t('preview:actions.unselect_mod')}
              className={`btn btn-circle btn-ghost hidden text-base-content/30 hover:bg-base-content/5 hover:text-base-content md:inline-flex transition-all duration-200 ${isScrolled ? 'btn-xs' : 'btn-sm'}`}
              title={t('preview:actions.close')}
            >
              <X size={isScrolled ? 16 : 18} />
            </button>
            <button
              onClick={() => setMobilePane('grid')}
              aria-label={t('preview:actions.close')}
              className={`btn btn-circle btn-ghost text-base-content/30 hover:text-base-content md:hidden transition-all duration-200 ${isScrolled ? 'btn-xs' : 'btn-sm'}`}
            >
              <X size={isScrolled ? 16 : 18} />
            </button>
          </div>
        </div>
      </div>

      <GallerySection
        images={images}
        currentImageIndex={currentImageIndex}
        isFetching={previewImagesQuery.isFetching}
        canEdit={!!activePath}
        isMutating={
          savePreviewImage.isPending || removePreviewImage.isPending || clearPreviewImages.isPending
        }
        onPrev={() => setCurrentImageIndex((prev) => (prev === 0 ? images.length - 1 : prev - 1))}
        onNext={() => setCurrentImageIndex((prev) => (prev + 1) % Math.max(images.length, 1))}
        onSelectIndex={setCurrentImageIndex}
        onPaste={() => {
          void pasteThumbnailFromClipboard();
        }}
        onImport={() => {
          if (!activePath) {
            toast.warning(t('preview:empty.no_mod_selected'));
            return;
          }
          importInputRef.current?.click();
        }}
        onRequestRemoveCurrent={() => {
          if (!currentImagePath) {
            toast.warning(t('preview:gallery.no_thumbnail_selected'));
            return;
          }
          setConfirmRemoveOpen(true);
        }}
        onRequestClearAll={() => {
          if (images.length === 0) {
            toast.warning(t('preview:gallery.no_thumbnails_to_clear'));
            return;
          }
          setConfirmClearOpen(true);
        }}
      />

      <MetadataSection
        activePath={activePath}
        authorDraft={authorDraft}
        versionDraft={versionDraft}
        descriptionDraft={descriptionDraft}
        metadataDirty={metadataDirty}
        onAuthorChange={setAuthorDraft}
        onVersionChange={setVersionDraft}
        onDescriptionChange={setDescriptionDraft}
        onDiscard={discardMetadata}
      />

      <IniEditorSection
        activePath={activePath}
        activeObjectName={selectedFolder?.name}
        selectedFolderName={selectedFolder?.folder_name}
        sections={keyBindSections}
        openSectionIds={openSectionIds}
        draftByField={draftByField}
        fieldErrors={fieldErrors}
        conflictingKeys={conflictingKeys}
        editorDirty={hasUnsavedEditorChanges}
        isSaving={writeModIni.isPending}
        onToggleSection={requestToggleSection}
        onFieldChange={updateEditorField}
        onSave={async () => {
          const success = await saveEditor();
          return success !== false; // Assuming saveEditor throws or returns false on fail. Assuming it succeeds if no generic error.
          // Wait, saveEditor is a void function that toasts on error. We can just always close it, or check if editorDirty is false after.
        }}
        onDiscard={discardEditor}
      />

      <div className="mt-auto pt-6">
        <button
          className="btn btn-outline btn-sm w-full gap-2"
          onClick={async () => {
            if (!activePath) {
              toast.warning('Select a mod folder first.');
              return;
            }

            try {
              if (activeGame?.id) {
                await commands.openInExplorer({ gameId: activeGame.id, path: activePath });
              }
            } catch (error) {
              toast.error(`Cannot open folder location: ${toErrorMessage(error)}`);
            }
          }}
          disabled={!activePath}
        >
          <Info size={16} />
          {t('preview:actions.view_location')}
        </button>
      </div>

      <PreviewPanelModals
        moveDialog={actions.moveDialog}
        closeMoveDialog={actions.closeMoveDialog}
        handleMoveToObject={actions.handleMoveToObject}
        objectId={selectedFolder?.id ?? undefined}
        deleteConfirm={actions.deleteConfirm}
        setDeleteConfirm={actions.setDeleteConfirm}
        handleDeleteConfirm={actions.handleDeleteConfirm}
        renameDialog={actions.renameDialog}
        handleRenameCancel={actions.handleRenameCancel}
        handleRenameSubmit={actions.handleRenameSubmit}
        duplicateWarning={actions.duplicateWarning}
        handleDuplicateForceEnable={actions.handleDuplicateForceEnable}
        handleDuplicateEnableOnly={actions.handleDuplicateEnableOnly}
        handleDuplicateCancel={actions.handleDuplicateCancel}
        pinSafeDialog={actions.pinSafeDialog}
        handleToggleSafeCancel={actions.handleToggleSafeCancel}
        handleToggleSafeSubmit={actions.handleToggleSafeSubmit}
      />
    </div>
  );
}
