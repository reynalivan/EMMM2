import { useEffect, useMemo, useState } from 'react';
import { useModFolders, useToggleMod } from '../../../hooks/useFolders';
import { useAppStore } from '../../../stores/useAppStore';
import { toast } from '../../../stores/useToastStore';
import {
  useClearPreviewImages,
  useAllModIniDocuments,
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

  const [titleDraft, setTitleDraft] = useState('');
  const [descriptionDraft, setDescriptionDraft] = useState('');
  const [syncedTitle, setSyncedTitle] = useState('');
  const [syncedDescription, setSyncedDescription] = useState('');

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

  const iniFiles = iniFilesQuery.data ?? [];
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

  const metadataDirty =
    !!activePath && (titleDraft !== syncedTitle || descriptionDraft !== syncedDescription);

  const images = previewImagesQuery.data ?? [];

  useEffect(() => {
    if (!activePath) {
      setTitleDraft('');
      setDescriptionDraft('');
      setSyncedTitle('');
      setSyncedDescription('');
      return;
    }

    const nextTitle = modInfoQuery.data?.actual_name ?? selectedFolder?.name ?? '';
    const nextDescription = modInfoQuery.data?.description ?? '';
    setTitleDraft(nextTitle);
    setDescriptionDraft(nextDescription);
    setSyncedTitle(nextTitle);
    setSyncedDescription(nextDescription);
  }, [
    activePath,
    modInfoQuery.data?.actual_name,
    modInfoQuery.data?.description,
    selectedFolder?.name,
  ]);

  useEffect(() => {
    if (!activePath || !metadataDirty) return;

    const timer = setTimeout(() => {
      updateModInfo
        .mutateAsync({
          folderPath: activePath,
          update: {
            actual_name: titleDraft,
            description: descriptionDraft,
          },
        })
        .then((saved) => {
          setSyncedTitle(saved.actual_name);
          setSyncedDescription(saved.description);
        })
        .catch((error) => {
          if (error.message?.includes('permission') || error.message?.includes('EACCES')) {
            toast.error('Permission denied. Cannot save metadata.');
          } else {
            toast.error(`Autosave failed: ${toErrorMessage(error)}`);
          }
        });
    }, 500);

    return () => clearTimeout(timer);
  }, [titleDraft, descriptionDraft, activePath, metadataDirty]);

  useEffect(() => {
    if (externalSelectedPath === activePath) {
      return;
    }

    if (hasUnsavedEditorChanges) {
      if (activePath) {
        useAppStore.setState({
          gridSelection: new Set([activePath]),
          mobileActivePane: 'details',
        });
      }
      setPendingTransition({ kind: 'mod', path: externalSelectedPath });
      setShowUnsavedModal(true);
      return;
    }

    setActivePath(externalSelectedPath);
  }, [externalSelectedPath, activePath, hasUnsavedEditorChanges]);

  useEffect(() => {
    setCurrentImageIndex(0);
  }, [activePath, images.length]);

  useEffect(() => {
    const nextInitialMap = toFieldValueMap(allKeyBindFields);
    if (Object.keys(draftByField).length > 0 && hasUnsavedEditorChanges) {
      return;
    }

    setInitialByField(nextInitialMap);
    setDraftByField(nextInitialMap);
    setFieldErrors({});
  }, [allKeyBindFields, hasUnsavedEditorChanges, draftByField]);

  useEffect(() => {
    setOpenSectionIds((prev) => {
      const validIds = new Set(keyBindSections.map((section) => section.id));
      const next = new Set(Array.from(prev).filter((id) => validIds.has(id)));
      if (next.size === 0 && keyBindSections[0]) {
        next.add(keyBindSections[0].id);
      }
      return next;
    });
  }, [keyBindSections]);

  const applyPendingTransition = (transition: PendingTransition) => {
    if (!transition) return;
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

  const saveMetadata = async () => {
    if (!activePath || !metadataDirty) return;

    try {
      const saved = await updateModInfo.mutateAsync({
        folderPath: activePath,
        update: {
          actual_name: titleDraft,
          description: descriptionDraft,
        },
      });
      setSyncedTitle(saved.actual_name);
      setSyncedDescription(saved.description);
      toast.success('Metadata saved.');
    } catch (error) {
      toast.error(`Cannot save metadata: ${toErrorMessage(error)}`);
    }
  };

  const discardMetadata = () => {
    setTitleDraft(syncedTitle);
    setDescriptionDraft(syncedDescription);
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
      if (isOpen) next.delete(sectionId);
      else next.add(sectionId);
      return next;
    });
  };

  const updateEditorField = (fieldId: string, value: string) => {
    setDraftByField((prev) => ({ ...prev, [fieldId]: value }));
    setFieldErrors((prev) => {
      if (!prev[fieldId]) {
        return prev;
      }
      const next = { ...prev };
      delete next[fieldId];
      return next;
    });
  };

  return {
    activePath,
    selectedFolder,
    images,
    currentImageIndex,
    setCurrentImageIndex,
    titleDraft,
    descriptionDraft,
    setTitleDraft,
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
    setActivePath,
  };
}
