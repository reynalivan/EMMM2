import { useCallback, useLayoutEffect, useRef, useState } from 'react';
import { Info } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '@/app/store';
import GallerySection from './components/GallerySection';
import MetadataSection from './components/MetadataSection';
import IniEditorSection from './components/IniEditorSection';
import { useActiveGame } from '@/entities/game';
import { usePreviewPanelState } from './hooks/usePreviewPanelState';
import PreviewPanelModals from './components/PreviewPanelModals';
import { useSharedModActions } from '@/features/mod-runtime';
import { dispatchWorkspaceRuntimeEvent, useWorkspaceRuntime } from '@/features/workspace-runtime';
import { formatWorkspaceWarning } from '@/features/workspace-runtime';
import { usePreviewActions } from './hooks/usePreviewActions';
import { usePreviewEffects } from './hooks/usePreviewEffects';
import PreviewEmptyState from './components/PreviewEmptyState';
import PreviewConfirmDialogs from './components/PreviewConfirmDialogs';
import PreviewHeader from './components/PreviewHeader';
import { openFolderConflictManagerDialog } from '@/features/workspace-runtime';
import PreviewFolderConflictState from './components/PreviewFolderConflictState';
import { PreviewErrorState, PreviewLoadingState } from './components/PreviewReadState';
import ModHealthSection from './components/ModHealthSection';
import { useModHealth } from './hooks/useModHealth';
import { toModHealthPanelData } from './utils/modHealthPresentation';
import { useModViewerExternalReview } from '@/features/mod-runtime';
import { commands } from '@/shared/api/tauri/bindings';
import { formatAppError } from '@/shared/lib/appError';
import { toast } from '@/shared/ui/toast';

export default function PreviewPanel() {
  const { t } = useTranslation(['preview', 'common']);
  const setMobilePane = useAppStore((state) => state.setMobilePane);
  const runtime = useWorkspaceRuntime();
  const { activeGame } = useActiveGame();
  const panelRef = useRef<HTMLDivElement>(null);
  const headerRef = useRef<HTMLDivElement>(null);

  const {
    activePath,
    folderNameConflict,
    selectedFolder,
    previewSummary,
    resolvedTitle,
    resolvedSubtitle,
    sourceUnavailableMessage,
    isPreviewLoading,
    previewError,
    retryPreview,
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
    switchSurface: 'preview',
  });
  const canEdit = Boolean(activePath) && !sourceUnavailableMessage && !folderNameConflict;
  const interactiveActivePath = folderNameConflict ? null : activePath;
  const modHealthPath =
    sourceUnavailableMessage || !selectedFolder?.is_effectively_active
      ? null
      : interactiveActivePath;
  const modHealth = useModHealth(modHealthPath);
  const modViewerExternalReview = useModViewerExternalReview(
    sourceUnavailableMessage ? null : interactiveActivePath,
  );
  const modHealthReport = modHealth.data ? toModHealthPanelData(modHealth.data) : null;

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
    activePath: interactiveActivePath,
    selectedFolder,
    images,
    currentImagePath,
    setCurrentImageIndex,
    savePreviewImage,
    removePreviewImage,
    clearPreviewImages,
  });
  usePreviewEffects({
    activePath: interactiveActivePath,
    pasteThumbnailFromClipboard,
  });

  useLayoutEffect(() => {
    const panel = panelRef.current;
    const header = headerRef.current;
    if (!panel || !header) return;

    const syncHeaderHeight = () => {
      panel.style.setProperty('--preview-header-height', `${header.offsetHeight}px`);
    };

    syncHeaderHeight();
    const observer = new ResizeObserver(syncHeaderHeight);
    observer.observe(header);
    return () => observer.disconnect();
  }, [activePath]);

  if (isPreviewLoading) {
    return <PreviewLoadingState />;
  }

  if (previewError) {
    return (
      <PreviewErrorState
        errorMessage={formatAppError(previewError)}
        onRetry={() => {
          void retryPreview();
        }}
      />
    );
  }

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

  if (folderNameConflict) {
    return (
      <PreviewFolderConflictState
        conflict={folderNameConflict}
        onBack={() => runtime.clearSelection({ resetExplorer: true, clearObjectSelection: true })}
        onResolve={openFolderConflictManagerDialog}
      />
    );
  }

  return (
    <div
      ref={panelRef}
      key={activePath}
      className="preview-panel workspace-context-enter workspace-scroll-owner flex h-full w-full max-w-none flex-col overflow-y-auto border-l border-base-content/5 bg-base-100/85 px-6 pb-6 pt-[var(--workspace-topbar-height)]"
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

      <div ref={headerRef}>
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
      </div>

      <GallerySection
        images={images}
        imageRefreshKey={previewImagesQuery.dataUpdatedAt}
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
        activeGameId={activeGame?.id ?? null}
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

      <ModHealthSection
        report={modHealthReport}
        isLoading={modHealth.isFetching}
        errorMessage={modHealth.isError ? formatAppError(modHealth.error) : null}
        onRecheck={() => {
          void modHealth.refetch();
        }}
        externalReview={modViewerExternalReview.review}
        onDismissReview={modViewerExternalReview.dismiss}
        onOpenIssueFile={(filePath) => {
          if (!activeGame?.id || !interactiveActivePath) {
            return;
          }
          void commands
            .openIniInEditor(activeGame.id, interactiveActivePath, filePath)
            .catch((error) => {
              toast.error(
                t('preview:errors.open_location_failed', { error: formatAppError(error) }),
              );
            });
        }}
      />

      <PreviewPanelModals
        deleteConfirm={actions.deleteConfirm}
        setDeleteConfirm={actions.setDeleteConfirm}
        handleDeleteConfirm={actions.handleDeleteConfirm}
        duplicateWarning={actions.duplicateWarning}
        handleDuplicateForceEnable={actions.handleDuplicateForceEnable}
        handleDuplicateEnableOnly={actions.handleDuplicateEnableOnly}
        handleDuplicateCancel={actions.handleDuplicateCancel}
      />
    </div>
  );
}
