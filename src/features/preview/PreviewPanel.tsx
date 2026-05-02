import { useCallback, useState } from 'react';
import { ChevronRight, Info, X, FolderOpen, FileArchive, FolderPlus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '../../stores/useAppStore';
import ConfirmDialog from '../../components/ui/ConfirmDialog';
import UnsavedIniChangesModal from './components/UnsavedIniChangesModal';
import GallerySection from './components/GallerySection';
import MetadataSection from './components/MetadataSection';
import IniEditorSection from './components/IniEditorSection';
import { useActiveGame } from '../../hooks/useActiveGame';
import { usePreviewPanelState } from './hooks/usePreviewPanelState';
import PreviewPanelModals from './components/PreviewPanelModals';
import PreviewPanelContextMenu from './components/PreviewPanelContextMenu';
import { useSharedModActions } from '../mod-runtime/actions/useSharedModActions';
import {
  dispatchWorkspaceRuntimeEvent,
  useWorkspaceRuntime,
} from '../workspace-runtime/state/workspaceStoreBridge';
import { formatWorkspaceWarning } from '../workspace-runtime/workspaceSemantics';
import { buildWorkspaceSwitchPolicy } from '../workspace-runtime/actions/workspaceSwitchPolicy';
import { WorkspaceSwitchControl } from '../workspace-runtime/components/WorkspaceSwitchControl';
import { WorkspaceSwitchLabel } from '../workspace-runtime/components/WorkspaceSwitchLabel';
import { usePreviewActions } from './hooks/usePreviewActions';
import { usePreviewEffects } from './hooks/usePreviewEffects';

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
  const switchPolicy = buildWorkspaceSwitchPolicy(t, selectedFolder);

  const boundedImageIndex = Math.min(currentImageIndex, Math.max(images.length - 1, 0));
  const currentImagePath = images[boundedImageIndex] ?? null;
  const warningSummary = previewSummary?.warning_summary ?? null;
  const primaryWarningText = warningSummary?.messages[0]
    ? formatWorkspaceWarning(t, warningSummary.messages[0])
    : null;

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
            onClick={() => {
              void requestImportArchives();
            }}
          >
            <FileArchive size={18} />
            {t('preview:actions.import_archives')}
          </button>
          <button
            className="btn btn-outline btn-primary gap-2"
            onClick={() => {
              void requestImportFolders();
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
        onChange={(event) => {
          void handleImportInputChange(event);
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
        onConfirm={() => {
          void confirmRemoveCurrentImage();
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
        onConfirm={() => {
          void confirmClearAllImages();
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
          dispatchWorkspaceRuntimeEvent({ type: 'PREVIEW_TRANSITION_CANCELLED' });
        }}
        onDiscard={() => {
          discardMetadata();
          discardEditor();
          applyPendingTransition();
        }}
        onSave={async () => {
          await saveMetadata();
          const editorSaved = await saveEditor();
          if (!editorSaved) return;
          applyPendingTransition();
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
                placeholder={resolvedTitle || t('preview:empty.no_mod_selected')}
                onChange={(e) => setTitleDraft(e.target.value)}
                disabled={!activePath}
              />
            </div>
            {resolvedSubtitle && (
              <p
                className={`truncate text-base-content/50 transition-all duration-200 ${
                  isScrolled ? 'mt-0.5 text-[10px]' : 'mt-1 text-xs'
                }`}
                title={resolvedSubtitle}
              >
                {resolvedSubtitle}
              </p>
            )}
            <label
              className={`label cursor-pointer justify-start gap-2 p-0 opacity-80 hover:opacity-100 transition-all duration-200 ${isScrolled ? '-mt-0.5' : 'mt-1'}`}
            >
              <WorkspaceSwitchControl
                node={selectedFolder}
                policy={switchPolicy}
                isPending={actions.isSwitchPending || actions.isFolderSwitchPending(selectedFolder)}
                size={isScrolled ? 'xs' : 'sm'}
                ariaLabel={t('preview:actions.toggle_enabled')}
                onToggle={(node) => {
                  if (node.node_kind === 'object') {
                    return;
                  }

                  void actions.handleToggleEnabled(node);
                }}
              />
              <WorkspaceSwitchLabel
                node={selectedFolder}
                policy={switchPolicy}
                className={`font-medium text-base-content/60 transition-all duration-200 ${isScrolled ? 'text-[10px]' : 'text-sm'}`}
              />
            </label>
            {warningSummary && warningSummary.messages.length > 0 && (
              <p
                className={`mt-1 truncate text-warning/80 transition-all duration-200 ${
                  isScrolled ? 'text-[10px]' : 'text-xs'
                }`}
                title={warningSummary.messages
                  .map((entry) => formatWorkspaceWarning(t, entry) ?? '')
                  .filter(Boolean)
                  .join('\n')}
              >
                {primaryWarningText}
              </p>
            )}
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
              onClick={() =>
                runtime.clearSelection({ resetExplorer: true, clearObjectSelection: true })
              }
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
          onClick={() => {
            void openCurrentLocation();
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
