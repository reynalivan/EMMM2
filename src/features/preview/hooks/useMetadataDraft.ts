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

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function useMetadataDraft({
  activePath,
  fallbackTitle,
  source,
  onSave,
}: UseMetadataDraftParams) {
  const [titleDraft, setTitleDraft] = useState('');
  const [authorDraft, setAuthorDraft] = useState('');
  const [versionDraft, setVersionDraft] = useState('');
  const [descriptionDraft, setDescriptionDraft] = useState('');

  const [syncedTitle, setSyncedTitle] = useState('');
  const [syncedAuthor, setSyncedAuthor] = useState('');
  const [syncedVersion, setSyncedVersion] = useState('');
  const [syncedDescription, setSyncedDescription] = useState('');

  const sourceTitle = source?.actual_name ?? fallbackTitle;
  const sourceAuthor = source?.author ?? 'Unknown';
  const sourceVersion = source?.version ?? '1.0';
  const sourceDescription = source?.description ?? '';

  useEffect(() => {
    if (!activePath) {
      setTitleDraft('');
      setAuthorDraft('');
      setVersionDraft('');
      setDescriptionDraft('');
      setSyncedTitle('');
      setSyncedAuthor('');
      setSyncedVersion('');
      setSyncedDescription('');
      return;
    }

    setTitleDraft(sourceTitle);
    setAuthorDraft(sourceAuthor);
    setVersionDraft(sourceVersion);
    setDescriptionDraft(sourceDescription);
    setSyncedTitle(sourceTitle);
    setSyncedAuthor(sourceAuthor);
    setSyncedVersion(sourceVersion);
    setSyncedDescription(sourceDescription);
  }, [activePath, sourceAuthor, sourceDescription, sourceTitle, sourceVersion]);

  const metadataDirty = useMemo(
    () =>
      !!activePath &&
      (titleDraft !== syncedTitle ||
        authorDraft !== syncedAuthor ||
        versionDraft !== syncedVersion ||
        descriptionDraft !== syncedDescription),
    [
      activePath,
      titleDraft,
      syncedTitle,
      authorDraft,
      syncedAuthor,
      versionDraft,
      syncedVersion,
      descriptionDraft,
      syncedDescription,
    ],
  );

  const changedFields = useMemo<MetadataFieldChange[]>(() => {
    if (!metadataDirty) {
      return [];
    }

    const changes: MetadataFieldChange[] = [];
    if (titleDraft !== sourceTitle) {
      changes.push({
        label: 'Title',
        oldValue: sourceTitle,
        newValue: titleDraft,
      });
    }
    if (authorDraft !== sourceAuthor) {
      changes.push({
        label: 'Author',
        oldValue: sourceAuthor,
        newValue: authorDraft,
      });
    }
    if (versionDraft !== sourceVersion) {
      changes.push({
        label: 'Version',
        oldValue: sourceVersion,
        newValue: versionDraft,
      });
    }
    if (descriptionDraft !== sourceDescription) {
      changes.push({
        label: 'Description',
        oldValue: sourceDescription,
        newValue: descriptionDraft,
      });
    }

    return changes;
  }, [
    authorDraft,
    descriptionDraft,
    metadataDirty,
    sourceAuthor,
    sourceDescription,
    sourceTitle,
    sourceVersion,
    titleDraft,
    versionDraft,
  ]);

  const saveMetadata = useCallback(async () => {
    if (!activePath || !metadataDirty) {
      return;
    }

    if (titleDraft.trim() === '') {
      toast.warning('Title cannot be empty');
      return;
    }

    try {
      const saved = await onSave(activePath, {
        actual_name: titleDraft,
        author: authorDraft,
        version: versionDraft,
        description: descriptionDraft,
      });
      setSyncedTitle(saved.actual_name);
      setSyncedAuthor(saved.author);
      setSyncedVersion(saved.version);
      setSyncedDescription(saved.description);
      toast.success('Metadata auto-saved.');
    } catch (error) {
      toast.error(`Cannot save metadata: ${toErrorMessage(error)}`);
    }
  }, [activePath, authorDraft, descriptionDraft, metadataDirty, onSave, titleDraft, versionDraft]);

  // Auto-save with long debounce
  useEffect(() => {
    if (!metadataDirty || !activePath) {
      return;
    }

    // validasi kalau isinya nol > akan diabaikan
    if (titleDraft.trim() === '') {
      return;
    }

    const timer = setTimeout(() => {
      void saveMetadata();
    }, 2500); // 2.5 seconds duration to allow reverting back

    return () => clearTimeout(timer);
  }, [activePath, metadataDirty, saveMetadata, titleDraft]);

  const discardMetadata = useCallback(() => {
    setTitleDraft(syncedTitle);
    setAuthorDraft(syncedAuthor);
    setVersionDraft(syncedVersion);
    setDescriptionDraft(syncedDescription);
  }, [syncedAuthor, syncedDescription, syncedTitle, syncedVersion]);

  return {
    titleDraft,
    authorDraft,
    versionDraft,
    descriptionDraft,
    setTitleDraft,
    setAuthorDraft,
    setVersionDraft,
    setDescriptionDraft,
    metadataDirty,
    changedFields,
    saveMetadata,
    discardMetadata,
  };
}
