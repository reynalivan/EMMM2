import { formatAppError } from '../../../shared/lib/appError';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from '@/shared/ui/toast';

export interface MetadataDraftValues {
  actual_name: string;
  author: string;
  version: string;
  description: string;
}

export interface MetadataFieldChange {
  label: string;
  oldValue: string;
  newValue: string;
}

interface UseMetadataDraftParams {
  activePath: string | null;
  selectedPath: string | null;
  gameId: string | null;
  filesystemIdentity: string | null;
  fallbackTitle: string;
  source: Partial<MetadataDraftValues> | null | undefined;
  onSave: (activePath: string, draft: MetadataDraftValues) => Promise<MetadataDraftValues>;
}

interface MetadataDraftState {
  draft: MetadataDraftValues;
  synced: MetadataDraftValues;
  isEditing: boolean;
}

interface MetadataSelection {
  gameId: string | null;
  filesystemIdentity: string | null;
  ownerPath: string | null;
  selectedPath: string | null;
  activePath: string | null;
  revision: number;
  save: { values: MetadataDraftValues; revision: number } | null;
}

const FIELD_LABELS: Record<keyof MetadataDraftValues, string> = {
  actual_name: 'Title',
  author: 'Author',
  version: 'Version',
  description: 'Description',
};

const FIELD_KEYS = Object.keys(FIELD_LABELS) as Array<keyof MetadataDraftValues>;

const EMPTY_DRAFT: MetadataDraftValues = {
  actual_name: '',
  author: '',
  version: '',
  description: '',
};

export function useMetadataDraft({
  activePath,
  selectedPath,
  gameId,
  filesystemIdentity,
  fallbackTitle,
  source,
  onSave,
}: UseMetadataDraftParams) {
  const { t } = useTranslation(['preview', 'common']);
  const [{ draft, synced, isEditing }, setState] = useState<MetadataDraftState>({
    draft: EMPTY_DRAFT,
    synced: EMPTY_DRAFT,
    isEditing: false,
  });
  const selectionRef = useRef<MetadataSelection>({
    gameId: null,
    filesystemIdentity: null,
    ownerPath: null,
    selectedPath: null,
    activePath: null,
    revision: 0,
    save: null,
  });

  const sourceTitle = source?.actual_name ?? fallbackTitle;
  const sourceAuthor = source?.author ?? 'Unknown';
  const sourceVersion = source?.version ?? '1.0';
  const sourceDescription = source?.description ?? '';

  const sourceValues = useMemo<MetadataDraftValues>(
    () => ({
      actual_name: sourceTitle,
      author: sourceAuthor,
      version: sourceVersion,
      description: sourceDescription,
    }),
    [sourceAuthor, sourceDescription, sourceTitle, sourceVersion],
  );

  useEffect(() => {
    const previousSelection = selectionRef.current;
    const sameOwner =
      gameId === previousSelection.gameId &&
      selectedPath !== null &&
      previousSelection.selectedPath !== null &&
      (!activePath ||
        (filesystemIdentity !== null
          ? filesystemIdentity === previousSelection.filesystemIdentity
          : previousSelection.filesystemIdentity === null &&
            activePath === previousSelection.ownerPath));
    if (!sameOwner) {
      selectionRef.current = {
        gameId,
        filesystemIdentity: activePath ? filesystemIdentity : null,
        ownerPath: activePath,
        selectedPath,
        activePath,
        revision: 0,
        save: null,
      };
    } else {
      if (
        previousSelection.activePath !== activePath ||
        previousSelection.selectedPath !== selectedPath
      ) {
        previousSelection.revision += 1;
      }
      previousSelection.activePath = activePath;
      previousSelection.selectedPath = selectedPath;
      if (activePath) previousSelection.ownerPath = activePath;
    }
    const pendingValues = selectionRef.current.save?.values;
    setState((previous) => {
      const next = selectedPath && activePath ? sourceValues : EMPTY_DRAFT;
      if (!sameOwner) return { draft: next, synced: next, isEditing: false };
      // A missing ready preview suspends writes; it is not a deselection.
      if (!activePath) return previous;

      const merged = { ...previous.draft };
      for (const key of FIELD_KEYS) {
        // A field reverted to its old baseline is still a newer edit than an in-flight save.
        if (
          previous.draft[key] === previous.synced[key] &&
          (!pendingValues || previous.draft[key] === pendingValues[key])
        ) {
          merged[key] = next[key];
        }
      }
      return { ...previous, draft: merged, synced: next };
    });
  }, [activePath, selectedPath, gameId, filesystemIdentity, sourceValues]);

  const metadataDirty = !!selectedPath && FIELD_KEYS.some((key) => draft[key] !== synced[key]);

  const changedFields = useMemo<MetadataFieldChange[]>(() => {
    if (!metadataDirty) {
      return [];
    }

    return FIELD_KEYS.filter((key) => draft[key] !== synced[key]).map((key) => ({
      label: FIELD_LABELS[key],
      oldValue: synced[key],
      newValue: draft[key],
    }));
  }, [draft, metadataDirty, synced]);

  const saveMetadata = useCallback(async () => {
    if (!activePath || !metadataDirty) {
      return;
    }

    if (draft.actual_name.trim() === '') {
      toast.warning(t('preview:metadata.title_required'));
      return;
    }

    const selection = selectionRef.current;
    if (
      selection.activePath !== activePath ||
      selection.gameId !== gameId ||
      selection.filesystemIdentity !== filesystemIdentity
    )
      return;
    const attempt = { values: draft, revision: selection.revision };
    selection.save = attempt;
    try {
      const saved = await onSave(activePath, draft);
      if (
        selectionRef.current !== selection ||
        selection.save !== attempt ||
        selection.revision !== attempt.revision
      )
        return;
      setState((previous) => {
        const merged = { ...previous.draft };
        for (const key of FIELD_KEYS) {
          if (previous.draft[key] === attempt.values[key]) merged[key] = saved[key];
        }
        return { ...previous, draft: merged, synced: saved };
      });
      toast.success(t('preview:metadata.auto_saved'));
    } catch (error) {
      if (
        selectionRef.current !== selection ||
        selection.save !== attempt ||
        selection.revision !== attempt.revision
      )
        return;
      toast.error(t('preview:metadata.save_error', { error: formatAppError(error) }));
    } finally {
      if (selection.save === attempt && selection.revision === attempt.revision)
        selection.save = null;
    }
  }, [activePath, gameId, filesystemIdentity, draft, metadataDirty, onSave, t]);

  // Auto-save with long debounce
  useEffect(() => {
    if (!metadataDirty || !activePath) {
      return;
    }

    // validasi kalau isinya nol > akan diabaikan
    if (draft.actual_name.trim() === '') {
      return;
    }

    const timer = setTimeout(() => {
      void saveMetadata();
    }, 2500); // 2.5 seconds duration to allow reverting back

    return () => clearTimeout(timer);
  }, [activePath, draft.actual_name, metadataDirty, saveMetadata]);

  const discardMetadata = useCallback(() => {
    setState((previous) => ({ ...previous, draft: previous.synced }));
  }, []);

  const setters = useMemo(
    () => ({
      setTitleDraft: (value: string) =>
        setState((prev) => ({ ...prev, draft: { ...prev.draft, actual_name: value } })),
      setAuthorDraft: (value: string) =>
        setState((prev) => ({ ...prev, draft: { ...prev.draft, author: value } })),
      setVersionDraft: (value: string) =>
        setState((prev) => ({ ...prev, draft: { ...prev.draft, version: value } })),
      setDescriptionDraft: (value: string) =>
        setState((prev) => ({ ...prev, draft: { ...prev.draft, description: value } })),
      setMetadataEditing: (value: boolean) => setState((prev) => ({ ...prev, isEditing: value })),
    }),
    [],
  );

  return {
    titleDraft: draft.actual_name,
    authorDraft: draft.author,
    versionDraft: draft.version,
    descriptionDraft: draft.description,
    ...setters,
    isMetadataEditing: isEditing,
    metadataDirty,
    changedFields,
    saveMetadata,
    discardMetadata,
  };
}
