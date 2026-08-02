import { formatAppError } from '../../../lib/appError';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { toast } from '../../../stores/useToastStore';

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
  fallbackTitle: string;
  source: Partial<MetadataDraftValues> | null | undefined;
  onSave: (activePath: string, draft: MetadataDraftValues) => Promise<MetadataDraftValues>;
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
  fallbackTitle,
  source,
  onSave,
}: UseMetadataDraftParams) {
  const [draft, setDraft] = useState(EMPTY_DRAFT);
  const [synced, setSynced] = useState(EMPTY_DRAFT);

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
    const next = activePath ? sourceValues : EMPTY_DRAFT;
    setDraft(next);
    setSynced(next);
  }, [activePath, sourceValues]);

  const metadataDirty = !!activePath && FIELD_KEYS.some((key) => draft[key] !== synced[key]);

  const changedFields = useMemo<MetadataFieldChange[]>(() => {
    if (!metadataDirty) {
      return [];
    }

    return FIELD_KEYS.filter((key) => draft[key] !== sourceValues[key]).map((key) => ({
      label: FIELD_LABELS[key],
      oldValue: sourceValues[key],
      newValue: draft[key],
    }));
  }, [draft, metadataDirty, sourceValues]);

  const saveMetadata = useCallback(async () => {
    if (!activePath || !metadataDirty) {
      return;
    }

    if (draft.actual_name.trim() === '') {
      toast.warning('Title cannot be empty');
      return;
    }

    try {
      setSynced(await onSave(activePath, draft));
      toast.success('Metadata auto-saved.');
    } catch (error) {
      toast.error(`Cannot save metadata: ${formatAppError(error)}`);
    }
  }, [activePath, draft, metadataDirty, onSave]);

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
    setDraft(synced);
  }, [synced]);

  const setters = useMemo(
    () => ({
      setTitleDraft: (value: string) => setDraft((prev) => ({ ...prev, actual_name: value })),
      setAuthorDraft: (value: string) => setDraft((prev) => ({ ...prev, author: value })),
      setVersionDraft: (value: string) => setDraft((prev) => ({ ...prev, version: value })),
      setDescriptionDraft: (value: string) => setDraft((prev) => ({ ...prev, description: value })),
    }),
    [],
  );

  return {
    titleDraft: draft.actual_name,
    authorDraft: draft.author,
    versionDraft: draft.version,
    descriptionDraft: draft.description,
    ...setters,
    metadataDirty,
    changedFields,
    saveMetadata,
    discardMetadata,
  };
}
