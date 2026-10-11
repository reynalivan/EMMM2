import { useEffect, useState } from 'react';
import { useMetadataDraft } from './useMetadataDraft';
import { useIniEditorState } from './useIniEditorState';
import {
  dispatchWorkspaceRuntimeEvent,
  useWorkspaceRuntimeSelector,
} from '@/features/workspace-runtime';
import { usePreviewRuntime } from './usePreviewRuntime';

export function usePreviewPanelState() {
  const [showHiddenImages, setShowHiddenImages] = useState(false);
  const {
    activeGameId,
    selectedPath,
    activePath,
    folderNameConflict,
    selectedFolder,
    previewSummary,
    resolvedTitle,
    resolvedSubtitle,
    sourceUnavailableMessage,
    availableObjects,
    isPreviewLoading,
    previewError,
    retryPreview,
    iniDocuments,
    iniDocumentsUpdatedAt = 0,
    images,
    hiddenImagePaths,
    hasVisibleImages,
    previewImagesQuery,
    updateModInfo,
    savePreviewImage,
    removePreviewImage,
    clearPreviewImages,
    restorePreviewImage,
    writeModIni,
  } = usePreviewRuntime(showHiddenImages);
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);
  const previewTransition = useWorkspaceRuntimeSelector((state) => state.previewTransition);
  const previewDirty = useWorkspaceRuntimeSelector((state) => state.previewDirty);
  const [currentImageIndex, setCurrentImageIndex] = useState(0);

  const editor = useIniEditorState({
    gameId: activeGameId,
    activePath,
    documents: iniDocuments,
    readVersion: iniDocumentsUpdatedAt,
    onWrite: writeModIni.mutateAsync,
  });

  const metaSource = previewSummary?.mod_info_summary ?? null;
  const {
    titleDraft,
    authorDraft,
    versionDraft,
    descriptionDraft,
    setTitleDraft,
    setAuthorDraft,
    setVersionDraft,
    setDescriptionDraft,
    isMetadataEditing,
    setMetadataEditing,
    metadataDirty,
    changedFields: changedMetadataFields,
    saveMetadata,
    discardMetadata,
  } = useMetadataDraft({
    activePath: sourceUnavailableMessage || folderNameConflict ? null : activePath,
    selectedPath,
    gameId: activeGameId,
    filesystemIdentity: selectedFolder?.filesystem_identity?.trim() || null,
    fallbackTitle: resolvedTitle ?? '',
    source: metaSource,
    onSave: async (folderPath, draft) => {
      await updateModInfo.mutateAsync({ folderPath, update: draft });
      return draft;
    },
  });

  const hasUnsavedChanges = editor.hasUnsavedEditorChanges || metadataDirty;

  useEffect(() => {
    if (previewDirty !== hasUnsavedChanges) {
      dispatchWorkspaceRuntimeEvent({ type: 'PREVIEW_DIRTY_CHANGED', dirty: hasUnsavedChanges });
    }
  }, [hasUnsavedChanges, previewDirty]);

  useEffect(() => {
    setCurrentImageIndex(0);
  }, [activePath, images.length, showHiddenImages]);

  useEffect(() => {
    setShowHiddenImages(false);
  }, [activeGameId, activePath]);

  const pendingTransition =
    previewTransition.kind === 'pending' &&
    previewTransition.pendingTarget.kind === 'collapseSection'
      ? previewTransition.pendingTarget
      : null;
  const showUnsavedModal = dialogState.kind === 'previewUnsavedChanges';

  const applyPendingTransition = () => {
    if (pendingTransition?.kind === 'collapseSection') {
      editor.collapseSection(pendingTransition.sectionId);
    }
    dispatchWorkspaceRuntimeEvent({ type: 'PREVIEW_TRANSITION_CONFIRMED' });
  };

  return {
    activePath,
    folderNameConflict,
    selectedFolder,
    previewSummary,
    resolvedTitle,
    resolvedSubtitle,
    sourceUnavailableMessage,
    availableObjects,
    isPreviewLoading,
    previewError,
    retryPreview,
    images,
    hiddenImagePaths,
    hasVisibleImages,
    showHiddenImages,
    setShowHiddenImages,
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
    isMetadataEditing,
    setMetadataEditing,
    metadataDirty,
    keyBindSections: editor.keyBindSections,
    openSectionIds: editor.openSectionIds,
    draftByField: editor.draftByField,
    fieldErrors: editor.fieldErrors,
    conflictingKeys: editor.conflictingKeys,
    hasUnsavedEditorChanges: editor.hasUnsavedEditorChanges,
    changedIniFields: editor.changedIniFields,
    changedMetadataFields,
    updateModInfo,
    savePreviewImage,
    removePreviewImage,
    clearPreviewImages,
    restorePreviewImage,
    writeModIni,
    previewImagesQuery,
    showUnsavedModal,
    pendingTransition,
    applyPendingTransition,
    saveMetadata,
    discardMetadata,
    saveEditor: editor.saveEditor,
    discardEditor: editor.discardEditor,
    requestToggleSection: editor.requestToggleSection,
    updateEditorField: editor.updateEditorField,
  };
}
