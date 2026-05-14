import { useCallback, useState } from 'react';
import { Info } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../../stores/useAppStore';
import GallerySection from './components/GallerySection';
import MetadataSection from './components/MetadataSection';
import IniEditorSection from './components/IniEditorSection';
import { useActiveGame } from '../../hooks/useActiveGame';
import { usePreviewPanelState } from './hooks/usePreviewPanelState';
import PreviewPanelModals from './components/PreviewPanelModals';
import { useSharedModActions } from '../mod-runtime/actions/useSharedModActions';
import {
  dispatchWorkspaceRuntimeEvent,
  useWorkspaceRuntime,
} from '../workspace-runtime/state/workspaceStoreBridge';
import { formatWorkspaceWarning } from '../workspace-runtime/workspaceSemantics';
import { usePreviewActions } from './hooks/usePreviewActions';
import { usePreviewEffects } from './hooks/usePreviewEffects';
import PreviewEmptyState from './components/PreviewEmptyState';
import PreviewConfirmDialogs from './components/PreviewConfirmDialogs';
import PreviewHeader from './components/PreviewHeader';

export default function PreviewPanel() {
  const { t } = useTranslation(['preview', 'common']);
  const setMobilePane = useAppStore((state) => state.setMobilePane);
  const runtime = useWorkspaceRuntime();
  const { activeGame } = useActiveGame();

  const {
    activePath,
    selectedFolder,
    previewSummary,
    resolvedTitle,
    resolvedSubtitle,
    sourceUnavailableMessage,
    availableObjects,
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
    showUnsavedModal,
    applyPendingTransition,
    saveMetadata,
    discardMetadata,
    saveEditor,
    discardEditor,
    requestToggleSection,
    updateEditorField,
  } = usePreviewPanelState();
  const actions = useSharedModActions({
    removeFromCurrentView: true,
    switchSurface: 'preview',
  });
  const canEdit = Boolean(activePath) && !sourceUnavailableMessage;

  const boundedImageIndex = Math.min(currentImageIndex, Math.max(images.length - 1, 0));
  const currentImagePath = images[boundedImageIndex] ?? null;
  const warningSummary = previewSummary?.warning_summary ?? null;
  const primaryWarningText = warningSummary?.messages[0]
    ? formatWorkspaceWarning(t, warningSummary.messages[0])
    : null;
  const warningTooltip =
    warningSummary?.messages
      .map((entry) => formatWorkspaceWarning(t, entry) ?? '')
      .filter(Boolean)
      .join('\n') || null;

  const [isScrolled, setIsScrolled] = useState(false);
  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    setIsScrolled(e.currentTarget.scrollTop > 10);
  }, []);
  const {
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
  } = usePreviewActions({
    activeGameId: activeGame?.id ?? null,
    activePath,
    selectedFolder,
    images,
    currentImagePath,
    setCurrentImageIndex,
    savePreviewImage,
    removePreviewImage,
    clearPreviewImages,
  });
  usePreviewEffects({
    activePath,
    pasteThumbnailFromClipboard,
  });

  if (!activePath) {
    return (
      <PreviewEmptyState
        sourceUnavailableMessage={sourceUnavailableMessage}
        onImportArchives={() => {
          void requestImportArchives();
        }}
        onImportFolders={() => {
          void requestImportFolders();
        }}
      />
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
        onChange={(event) => {
          void handleImportInputChange(event);
        }}
      />

      <PreviewConfirmDialogs
        confirmRemoveOpen={confirmRemoveOpen}
        confirmClearOpen={confirmClearOpen}
        showUnsavedModal={showUnsavedModal}
        isSaving={writeModIni.isPending}
        modName={titleDraft || selectedFolder?.name}
        categoryName={selectedFolder?.category ?? undefined}
        changedIniFields={changedIniFields}
        changedMetadataFields={changedMetadataFields}
        onCancelRemove={() => setConfirmRemoveOpen(false)}
        onConfirmRemove={() => {
          void confirmRemoveCurrentImage();
        }}
        onCancelClear={() => setConfirmClearOpen(false)}
        onConfirmClear={() => {
          void confirmClearAllImages();
        }}
        onCancelUnsaved={() => {
          dispatchWorkspaceRuntimeEvent({ type: 'PREVIEW_TRANSITION_CANCELLED' });
        }}
        onDiscardUnsaved={() => {
          discardMetadata();
          discardEditor();
          applyPendingTransition();
        }}
        onSaveUnsaved={async () => {
          await saveMetadata();
          const editorSaved = await saveEditor();
          if (!editorSaved) {
            return;
          }
          applyPendingTransition();
        }}
      />

      <PreviewHeader
        selectedFolder={selectedFolder}
        resolvedTitle={resolvedTitle}
        resolvedSubtitle={resolvedSubtitle}
        titleDraft={titleDraft}
        warningText={primaryWarningText}
        warningTooltip={warningTooltip}
        sourceUnavailableMessage={sourceUnavailableMessage}
        isScrolled={isScrolled}
        canEdit={canEdit}
        actions={actions}
        onTitleChange={setTitleDraft}
        onBackToGrid={() => setMobilePane('grid')}
        onClearSelection={() =>
          runtime.clearSelection({ resetExplorer: true, clearObjectSelection: true })
        }
      />

      <GallerySection
        images={images}
        currentImageIndex={currentImageIndex}
        isFetching={previewImagesQuery.isFetching}
        canEdit={canEdit}
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
          triggerThumbnailImport();
        }}
        onRequestRemoveCurrent={() => {
          requestRemoveCurrentImage();
        }}
        onRequestClearAll={() => {
          requestClearAllImages();
        }}
      />

      <MetadataSection
        activePath={activePath}
        authorDraft={authorDraft}
        versionDraft={versionDraft}
        descriptionDraft={descriptionDraft}
        metadataDirty={metadataDirty}
        canEdit={canEdit}
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
        canEdit={canEdit}
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
          onClick={() => {
            void openCurrentLocation();
          }}
          disabled={!canEdit}
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
        objects={availableObjects}
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
        activeContextDialog={actions.activeContextDialog}
        handleActiveContextCancel={actions.handleActiveContextCancel}
        handleActiveContextSubmit={actions.handleActiveContextSubmit}
      />
    </div>
  );
}
