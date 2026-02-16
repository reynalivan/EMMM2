import { useEffect, useMemo, useState } from 'react';
import { toast } from '../../../stores/useToastStore';

export interface MetadataDraftValues {
  actual_name: string;
  author: string;
  version: string;
  description: string;
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

  useEffect(() => {
    if (!activePath) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
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

    const nextTitle = source?.actual_name ?? fallbackTitle;
    const nextAuthor = source?.author ?? 'Unknown';
    const nextVersion = source?.version ?? '1.0';
    const nextDescription = source?.description ?? '';

    setTitleDraft(nextTitle);
    setAuthorDraft(nextAuthor);
    setVersionDraft(nextVersion);
    setDescriptionDraft(nextDescription);
    setSyncedTitle(nextTitle);
    setSyncedAuthor(nextAuthor);
    setSyncedVersion(nextVersion);
    setSyncedDescription(nextDescription);
  }, [
    activePath,
    source?.actual_name,
    source?.author,
    source?.version,
    source?.description,
    fallbackTitle,
  ]);

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

  const saveMetadata = async () => {
    if (!activePath || !metadataDirty) {
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
      toast.success('Metadata saved.');
    } catch (error) {
      toast.error(`Cannot save metadata: ${toErrorMessage(error)}`);
    }
  };

  const discardMetadata = () => {
    setTitleDraft(syncedTitle);
    setAuthorDraft(syncedAuthor);
    setVersionDraft(syncedVersion);
    setDescriptionDraft(syncedDescription);
  };

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
    saveMetadata,
    discardMetadata,
  };
}
