import { useEffect, useMemo, useState } from 'react';
import { validateKeyBinding } from '../keybindingValidator';
import { useModFolders, useToggleMod } from '../../../hooks/useFolders';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';
import {
  useAllModIniDocuments,
  useClearPreviewImages,
  useModInfo,
  useModIniFiles,
  usePreviewImages,
  useRemovePreviewImage,
  useSavePreviewImage,
  useSelectedModPath,
  useUpdateModInfoDetails,
  useWriteModIni,
} from './usePreviewData';
import {
  buildKeyBindSections,
  buildVariableInfoSummaries,
  toFieldValueMap,
  toIniWritePayload,
} from '../previewPanelUtils';
import { useMetadataDraft } from './useMetadataDraft';

type PendingTransition =
  | { kind: 'mod'; path: string | null }
  | { kind: 'collapse'; sectionId: string }
  | null;

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function usePreviewPanelState() {
  const explorerSubPath = useAppStore((state) => state.explorerSubPath);
  const externalSelectedPath = useSelectedModPath();

  const [activePath, setActivePath] = useState<string | null>(externalSelectedPath);
  const [pendingTransition, setPendingTransition] = useState<PendingTransition>(null);
  const [showUnsavedModal, setShowUnsavedModal] = useState(false);
  const [currentImageIndex, setCurrentImageIndex] = useState(0);
  const [activeIniTab, setActiveIniTab] = useState<'keybind' | 'information'>('keybind');

  const [draftByField, setDraftByField] = useState<Record<string, string>>({});
  const [initialByField, setInitialByField] = useState<Record<string, string>>({});
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [openSectionIds, setOpenSectionIds] = useState<Set<string>>(new Set());

  const modInfoQuery = useModInfo(activePath);
  const iniFilesQuery = useModIniFiles(activePath);
  const previewImagesQuery = usePreviewImages(activePath);

  const updateModInfo = useUpdateModInfoDetails();
  const savePreviewImage = useSavePreviewImage();
  const removePreviewImage = useRemovePreviewImage();
  const clearPreviewImages = useClearPreviewImages();
  const writeModIni = useWriteModIni();
  const toggleMod = useToggleMod();

  const { data: folders = [] } = useModFolders(explorerSubPath);
  const selectedFolder = useMemo(
    () => folders.find((folder) => folder.path === activePath) ?? null,
    [folders, activePath],
  );

  const iniFiles = useMemo(() => iniFilesQuery.data ?? [], [iniFilesQuery.data]);
  const allIniQueries = useAllModIniDocuments(activePath, iniFiles);
  const iniDocuments = useMemo(
    () =>
      iniFiles.map((file, index) => ({
        fileName: file.filename,
        document: allIniQueries[index]?.data,
      })),
    [iniFiles, allIniQueries],
  );

  const keyBindSections = useMemo(() => buildKeyBindSections(iniDocuments), [iniDocuments]);
  const allKeyBindFields = useMemo(
    () => keyBindSections.flatMap((section) => section.fields),
    [keyBindSections],
  );
  const variableSummaries = useMemo(() => buildVariableInfoSummaries(iniDocuments), [iniDocuments]);

  const hasUnsavedEditorChanges = useMemo(
    () =>
      allKeyBindFields.some(
        (field) => (draftByField[field.id] ?? '') !== (initialByField[field.id] ?? ''),
      ),
    [allKeyBindFields, draftByField, initialByField],
  );

  const images = useMemo(() => previewImagesQuery.data ?? [], [previewImagesQuery.data]);

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
    saveMetadata,
    discardMetadata,
  } = useMetadataDraft({
    activePath,
    fallbackTitle: selectedFolder?.name ?? '',
    source: modInfoQuery.data,
    onSave: async (folderPath, draft) =>
      updateModInfo.mutateAsync({
        folderPath,
        update: draft,
      }),
  });

  const hasUnsavedChanges = hasUnsavedEditorChanges || metadataDirty;

  useEffect(() => {
    if (externalSelectedPath === activePath) {
      return;
    }

    if (hasUnsavedChanges) {
      if (activePath) {
        useAppStore.setState({
          gridSelection: new Set([activePath]),
          mobileActivePane: 'details',
        });
      }

      // Defer state update to avoid "setState during render" warning
      setTimeout(() => {
        setPendingTransition({ kind: 'mod', path: externalSelectedPath });
        setShowUnsavedModal(true);
      }, 0);
      return;
    }

    // Defer state update (derived from props)
    setTimeout(() => {
      setActivePath(externalSelectedPath);
    }, 0);
  }, [externalSelectedPath, activePath, hasUnsavedChanges]);

  // Reset image index when active path or image count changes
  // Reset image index when active path or image count changes
  const [prevActivePathForImg, setPrevActivePathForImg] = useState(activePath);
  const [prevImgLen, setPrevImgLen] = useState(images.length);
  if (activePath !== prevActivePathForImg || images.length !== prevImgLen) {
    setPrevActivePathForImg(activePath);
    setPrevImgLen(images.length);
    setCurrentImageIndex(0);
  }

  // Stable identity for allKeyBindFields via JSON key
  const fieldIds = useMemo(() => allKeyBindFields.map((f) => f.id).join('\0'), [allKeyBindFields]);
  const [prevFieldIds, setPrevFieldIds] = useState(fieldIds);

  if (fieldIds !== prevFieldIds) {
    setPrevFieldIds(fieldIds);
    if (!hasUnsavedEditorChanges) {
      const nextInitialMap = toFieldValueMap(allKeyBindFields);
      setInitialByField(nextInitialMap);
      setDraftByField(nextInitialMap);
      setFieldErrors({});
    }
  }

  // Stable identity for keyBindSections via JSON key
  const sectionIds = useMemo(() => keyBindSections.map((s) => s.id).join('\0'), [keyBindSections]);
  const [prevSectionIds, setPrevSectionIds] = useState(sectionIds);

  if (sectionIds !== prevSectionIds) {
    setPrevSectionIds(sectionIds);
    const validIds = new Set(keyBindSections.map((section) => section.id));
    const next = new Set(Array.from(openSectionIds).filter((id) => validIds.has(id)));
    if (next.size === 0 && keyBindSections[0]) {
      next.add(keyBindSections[0].id);
    }
    setOpenSectionIds(next);
  }

  const applyPendingTransition = (transition: PendingTransition) => {
    if (!transition) {
      return;
    }
    if (transition.kind === 'mod') {
      setActivePath(transition.path);
      return;
    }

    setOpenSectionIds((prev) => {
      const next = new Set(prev);
      next.delete(transition.sectionId);
      return next;
    });
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

      await Promise.all(allIniQueries.map((query) => query.refetch()));
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
      setPendingTransition({ kind: 'collapse', sectionId });
      setShowUnsavedModal(true);
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
  };
}
