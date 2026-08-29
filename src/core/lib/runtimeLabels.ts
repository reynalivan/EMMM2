import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';

export type RuntimeLabels = {
  currentChanges: string;
};

type CollectionDisplayNameInput = {
  name: string | null | undefined;
  isUnsaved: boolean | null | undefined;
  labels: RuntimeLabels;
};

export function getCollectionDisplayName(input: CollectionDisplayNameInput): string {
  const normalizedName = input.name?.trim();
  if (!input.isUnsaved && normalizedName) {
    return normalizedName;
  }
  return input.labels.currentChanges;
}

export function useRuntimeLabels(): RuntimeLabels {
  const { t } = useTranslation('layout');
  return useMemo(() => ({ currentChanges: t('context.current_changes', 'Current changes') }), [t]);
}
