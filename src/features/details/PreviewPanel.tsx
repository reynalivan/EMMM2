import { useCallback, useEffect, useRef, useState } from 'react';
import { ChevronRight, Info, Trash2, X } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useAppStore } from '../../stores/useAppStore';
import { toast } from '../../stores/useToastStore';
import ConfirmDialog from '../../components/ui/ConfirmDialog';
import UnsavedIniChangesModal from './components/UnsavedIniChangesModal';
import GallerySection from './components/GallerySection';
import MetadataSection from './components/MetadataSection';
import IniEditorSection from './components/IniEditorSection';
import { usePreviewPanelState } from './hooks/usePreviewPanelState';

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export default function PreviewPanel() {
  const togglePreview = useAppStore((state) => state.togglePreview);
  const setMobilePane = useAppStore((state) => state.setMobilePane);

  const importInputRef = useRef<HTMLInputElement>(null);
  const [confirmRemoveOpen, setConfirmRemoveOpen] = useState(false);
  const [confirmClearOpen, setConfirmClearOpen] = useState(false);

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
    activeIniTab,
    setActiveIniTab,
    keyBindSections,
    openSectionIds,
    draftByField,
    fieldErrors,
    variableSummaries,
    hasUnsavedEditorChanges,
    updateModInfo,
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

  const boundedImageIndex = Math.min(currentImageIndex, Math.max(images.length - 1, 0));
  const currentImagePath = images[boundedImageIndex] ?? null;

  const pasteThumbnailFromClipboard = useCallback(async () => {
    if (!activePath) {
      toast.warning('Select a mod folder first.');
      return;
    }

    if (!navigator.clipboard?.read) {
      toast.error('Clipboard image paste is not supported in this environment.');
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
        toast.warning('Clipboard does not contain an image.');
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
      toast.success('Thumbnail pasted.');
    } catch (error) {
      toast.error(`Cannot paste image: ${toErrorMessage(error)}`);
    }
  }, [activePath, savePreviewImage, selectedFolder, previewImagesQuery, setCurrentImageIndex]);

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

  return (
    <div className="mx-auto flex h-full w-full max-w-[560px] flex-col overflow-y-auto border-l border-white/5 bg-base-100/30 p-6 backdrop-blur-md">
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
            toast.success('Thumbnail imported.');
          } catch (error) {
            toast.error(`Cannot import thumbnail: ${toErrorMessage(error)}`);
          }
        }}
      />

      <ConfirmDialog
        open={confirmRemoveOpen}
        title="Remove Thumbnail"
        message="This will permanently remove the currently selected thumbnail image."
        confirmLabel="Remove"
        cancelLabel="Cancel"
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
            toast.success('Thumbnail removed.');
          } catch (error) {
            toast.error(`Cannot remove thumbnail: ${toErrorMessage(error)}`);
          }
        }}
      />

      <ConfirmDialog
        open={confirmClearOpen}
        title="Clear All Thumbnails"
        message="This will permanently remove all discovered thumbnails in this mod folder."
        confirmLabel="Clear All"
        cancelLabel="Cancel"
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
            toast.success('All thumbnails cleared.');
          } catch (error) {
            toast.error(`Cannot clear thumbnails: ${toErrorMessage(error)}`);
          }
        }}
      />

      <UnsavedIniChangesModal
        open={showUnsavedModal}
        isSaving={writeModIni.isPending}
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

      <div className="mb-6 flex items-start justify-between">
        <div className="min-w-0 flex-1">
          <div className="mb-1 flex items-center gap-2">
            <button
              onClick={() => setMobilePane('grid')}
              aria-label="Back to grid"
              className="btn btn-circle btn-ghost btn-xs text-white/50 hover:text-white md:hidden"
            >
              <ChevronRight className="rotate-180" size={16} />
            </button>
            <h2 className="truncate text-xl font-bold tracking-tight text-white">
              {titleDraft || selectedFolder?.name || 'No mod selected'}
            </h2>
          </div>
          <label className="label cursor-pointer justify-start gap-2 p-0 opacity-80 hover:opacity-100">
            <input
              type="checkbox"
              aria-label="Toggle mod enabled status"
              className="toggle toggle-sm border-white/10 bg-base-300 checked:border-primary checked:bg-primary"
              checked={selectedFolder?.is_enabled ?? false}
              disabled={!selectedFolder || toggleMod.isPending}
              onChange={() => {
                if (!selectedFolder) return;
                toggleMod.mutate({ path: selectedFolder.path, enable: !selectedFolder.is_enabled });
              }}
            />
            <span className="text-sm font-medium text-white/60">
              {selectedFolder
                ? selectedFolder.is_enabled
                  ? 'Enabled'
                  : 'Disabled'
                : 'No active mod'}
            </span>
          </label>
        </div>

        <div className="ml-2 flex items-center gap-1">
          <button className="btn btn-circle btn-ghost btn-sm text-error/50 hover:bg-error/10 hover:text-error">
            <span className="sr-only">Delete mod</span>
            <Trash2 size={18} />
          </button>
          <button
            onClick={togglePreview}
            aria-label="Toggle preview panel"
            className="btn btn-circle btn-ghost btn-sm hidden text-white/30 hover:bg-white/5 hover:text-white md:inline-flex"
            title="Close Preview"
          >
            <ChevronRight size={18} />
          </button>
          <button
            onClick={() => setMobilePane('grid')}
            aria-label="Close details pane"
            className="btn btn-circle btn-ghost btn-sm text-white/30 hover:text-white md:hidden"
          >
            <X size={18} />
          </button>
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
            toast.warning('Select a mod folder first.');
            return;
          }
          importInputRef.current?.click();
        }}
        onRequestRemoveCurrent={() => {
          if (!currentImagePath) {
            toast.warning('No thumbnail selected to remove.');
            return;
          }
          setConfirmRemoveOpen(true);
        }}
        onRequestClearAll={() => {
          if (images.length === 0) {
            toast.warning('No thumbnails to clear.');
            return;
          }
          setConfirmClearOpen(true);
        }}
      />

      <MetadataSection
        activePath={activePath}
        titleDraft={titleDraft}
        authorDraft={authorDraft}
        versionDraft={versionDraft}
        descriptionDraft={descriptionDraft}
        metadataDirty={metadataDirty}
        isSaving={updateModInfo.isPending}
        onTitleChange={setTitleDraft}
        onAuthorChange={setAuthorDraft}
        onVersionChange={setVersionDraft}
        onDescriptionChange={setDescriptionDraft}
        onSave={() => void saveMetadata()}
        onDiscard={discardMetadata}
      />

      <IniEditorSection
        activePath={activePath}
        activeTab={activeIniTab}
        sections={keyBindSections}
        openSectionIds={openSectionIds}
        draftByField={draftByField}
        fieldErrors={fieldErrors}
        variableSummaries={variableSummaries}
        editorDirty={hasUnsavedEditorChanges}
        isSaving={writeModIni.isPending}
        onTabChange={setActiveIniTab}
        onToggleSection={requestToggleSection}
        onFieldChange={updateEditorField}
        onSave={() => void saveEditor()}
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
              await invoke('open_in_explorer', { path: activePath });
            } catch (error) {
              toast.error(`Cannot open folder location: ${toErrorMessage(error)}`);
            }
          }}
          disabled={!activePath}
        >
          <Info size={16} />
          View File Location
        </button>
      </div>
    </div>
  );
}
