import { useEffect, useMemo, useState } from 'react';
import { validateKeyBinding } from '../keybindingValidator';
import { toast } from '../../../stores/useToastStore';
import {
  buildKeyBindSections,
  getConflictingKeys,
  toFieldValueMap,
  toIniWritePayload,
} from '../previewPanelUtils';
import { useMetadataDraft } from './useMetadataDraft';
import {
  dispatchWorkspaceRuntimeEvent,
  useWorkspaceRuntimeSelector,
} from '../../workspace-runtime/state/workspaceStoreBridge';
import { usePreviewRuntime } from './usePreviewRuntime';

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function areFieldMapsEqual(
  left: Record<string, string>,
  right: Record<string, string>,
): boolean {
  const leftKeys = Object.keys(left);
  const rightKeys = Object.keys(right);
  if (leftKeys.length !== rightKeys.length) {
    return false;
  }

  return leftKeys.every((key) => left[key] === right[key]);
}

function areStringSetsEqual(left: Set<string>, right: Set<string>): boolean {
  if (left.size !== right.size) {
    return false;
  }

  for (const value of left) {
    if (!right.has(value)) {
      return false;
    }
  }

  return true;
}

export function usePreviewPanelState() {
  const {
    activePath,
    selectedFolder,
    previewSummary,
    resolvedTitle,
    resolvedSubtitle,
    availableObjects,
    iniDocuments,
    images,
    previewImagesQuery,
    updateModInfo,
    savePreviewImage,
    removePreviewImage,
    clearPreviewImages,
    writeModIni,
  } = usePreviewRuntime();
  const dialogState = useWorkspaceRuntimeSelector((state) => state.dialogState);
  const previewTransition = useWorkspaceRuntimeSelector((state) => state.previewTransition);
  const previewDirty = useWorkspaceRuntimeSelector((state) => state.previewDirty);
  const [currentImageIndex, setCurrentImageIndex] = useState(0);

  const [draftByField, setDraftByField] = useState<Record<string, string>>({});
  const [initialByField, setInitialByField] = useState<Record<string, string>>({});
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [openSectionIds, setOpenSectionIds] = useState<Set<string>>(new Set());

  const keyBindSections = useMemo(() => buildKeyBindSections(iniDocuments), [iniDocuments]);
  const allKeyBindFields = useMemo(
    () => keyBindSections.flatMap((group) => group.sections.flatMap((section) => section.fields)),
    [keyBindSections],
  );

  const conflictingKeys = useMemo(
    () => getConflictingKeys(keyBindSections, draftByField),
    [keyBindSections, draftByField],
  );

  const changedIniFields = useMemo(() => {
    return allKeyBindFields
      .filter((field) => (draftByField[field.id] ?? '') !== (initialByField[field.id] ?? ''))
      .map((field) => ({
        label: field.label || field.id,
        filename: field.id.split('::')[0] || 'Unknown INI',
        oldValue: initialByField[field.id] ?? '',
        newValue: draftByField[field.id] ?? '',
      }));
  }, [allKeyBindFields, draftByField, initialByField]);

  const hasUnsavedEditorChanges = changedIniFields.length > 0;

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
    metadataDirty,
    changedFields: changedMetadataFields,
    saveMetadata,
    discardMetadata,
  } = useMetadataDraft({
    activePath,
    fallbackTitle: resolvedTitle ?? '',
    source: metaSource,
    onSave: async (folderPath, draft) => {
      await updateModInfo.mutateAsync({ folderPath, update: draft });
      return draft;
    },
  });

  const hasUnsavedChanges = hasUnsavedEditorChanges || metadataDirty;

  useEffect(() => {
    if (previewDirty === hasUnsavedChanges) {
      return;
    }

    dispatchWorkspaceRuntimeEvent({ type: 'PREVIEW_DIRTY_CHANGED', dirty: hasUnsavedChanges });
  }, [hasUnsavedChanges, previewDirty]);

  useEffect(() => {
    setCurrentImageIndex(0);
  }, [activePath, images.length]);

  const fieldIds = useMemo(() => allKeyBindFields.map((f) => f.id).join('\0'), [allKeyBindFields]);
  useEffect(() => {
    if (hasUnsavedEditorChanges) {
      return;
    }

    const nextInitialMap = toFieldValueMap(allKeyBindFields);
    setInitialByField((prev) => (areFieldMapsEqual(prev, nextInitialMap) ? prev : nextInitialMap));
    setDraftByField((prev) => (areFieldMapsEqual(prev, nextInitialMap) ? prev : nextInitialMap));
    setFieldErrors((prev) => (Object.keys(prev).length === 0 ? prev : {}));
  }, [fieldIds, hasUnsavedEditorChanges]);

  const sectionIds = useMemo(() => keyBindSections.map((s) => s.id).join('\0'), [keyBindSections]);
  useEffect(() => {
    const validIds = new Set(keyBindSections.map((section) => section.id));
    setOpenSectionIds((prev) => {
      const next = new Set(Array.from(prev).filter((id) => validIds.has(id)));
      if (next.size === 0 && keyBindSections[0]) {
        next.add(keyBindSections[0].id);
      }
      return areStringSetsEqual(prev, next) ? prev : next;
    });
  }, [sectionIds]);

  const pendingTransition =
    previewTransition.kind === 'pending' &&
    previewTransition.pendingTarget.kind === 'collapseSection'
      ? previewTransition.pendingTarget
      : null;
  const showUnsavedModal = dialogState.kind === 'previewUnsavedChanges';

  const applyPendingTransition = () => {
    if (pendingTransition?.kind === 'collapseSection') {
      setOpenSectionIds((prev) => {
        const next = new Set(prev);
        next.delete(pendingTransition.sectionId);
        return next;
      });
      dispatchWorkspaceRuntimeEvent({ type: 'PREVIEW_TRANSITION_CONFIRMED' });
      return;
    }

    dispatchWorkspaceRuntimeEvent({ type: 'PREVIEW_TRANSITION_CONFIRMED' });
  };

  const saveEditor = async (): Promise<boolean> => {
    if (!activePath) {
      return true;
    }

    const payload = toIniWritePayload(allKeyBindFields, draftByField, initialByField);
    setFieldErrors(payload.fieldErrors);

    if (Object.keys(payload.fieldErrors).length > 0) {
      toast.error('Fix invalid INI values before saving.');
      return false;
    }

    const updatesEntries = Object.entries(payload.updatesByFile);
    if (updatesEntries.length === 0) {
      return true;
    }

    try {
      for (const [fileName, lineUpdates] of updatesEntries) {
        await writeModIni.mutateAsync({
          folderPath: activePath,
          fileName,
          lineUpdates,
        });
      }

      setInitialByField({ ...draftByField });
      setDraftByField({ ...draftByField });
      setFieldErrors({});
      toast.success('INI editor saved.');
      return true;
    } catch (error) {
      toast.error(`Cannot save INI: ${toErrorMessage(error)}`);
      return false;
    }
  };

  const discardEditor = () => {
    setDraftByField({ ...initialByField });
    setFieldErrors({});
  };

  const requestToggleSection = (sectionId: string) => {
    const isOpen = openSectionIds.has(sectionId);
    if (isOpen && hasUnsavedEditorChanges) {
      dispatchWorkspaceRuntimeEvent({
        type: 'PREVIEW_TRANSITION_REQUESTED',
        target: { kind: 'collapseSection', sectionId },
      });
      return;
    }

    setOpenSectionIds((prev) => {
      const next = new Set(prev);
      if (isOpen) {
        next.delete(sectionId);
      } else {
        next.add(sectionId);
      }
      return next;
    });
  };

  const updateEditorField = (fieldId: string, value: string) => {
    setDraftByField((prev) => ({ ...prev, [fieldId]: value }));

    // Live keybinding validation for key/back fields
    const field = allKeyBindFields.find((f) => f.id === fieldId);
    if (field && (field.label === 'key' || field.label === 'back')) {
      const kbError = value.trim() ? validateKeyBinding(value) : null;
      setFieldErrors((prev) => {
        if (kbError) {
          return { ...prev, [fieldId]: kbError };
        }
        if (!prev[fieldId]) return prev;
        const next = { ...prev };
        delete next[fieldId];
        return next;
      });
    } else {
      setFieldErrors((prev) => {
        if (!prev[fieldId]) return prev;
        const next = { ...prev };
        delete next[fieldId];
        return next;
      });
    }
  };

  return {
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
    updateModInfo,
    savePreviewImage,
    removePreviewImage,
    clearPreviewImages,
    writeModIni,
    previewImagesQuery,
    showUnsavedModal,
    pendingTransition,
    applyPendingTransition,
    saveMetadata,
    discardMetadata,
    saveEditor,
    discardEditor,
    requestToggleSection,
    updateEditorField,
  };
}
