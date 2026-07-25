// Shared display labels for collection and runtime snapshot names.
import { useTranslation } from 'react-i18next';
import { useMemo } from 'react';

export type UnsavedCollectionLabels = {
  safeLabel: string;
  unsafeLabel: string;
};

type CollectionDisplayNameInput = {
  name: string | null | undefined;
  isUnsaved: boolean | null | undefined;
  isSafe: boolean | null | undefined;
  labels: UnsavedCollectionLabels;
};

type CorridorStateNameInput = {
  stateName: string | null | undefined;
  isUnsaved: boolean | null | undefined;
  isSafe: boolean | null | undefined;
  labels: UnsavedCollectionLabels;
};

export function getUnsavedCollectionLabel(
  isSafe: boolean | null | undefined,
  labels: UnsavedCollectionLabels,
): string {
  return isSafe === false ? labels.unsafeLabel : labels.safeLabel;
}

export function getCollectionDisplayName(input: CollectionDisplayNameInput): string {
  if (input.isUnsaved) {
    return getUnsavedCollectionLabel(input.isSafe, input.labels);
  }

  const normalizedName = input.name?.trim();
  if (normalizedName) {
    return normalizedName;
  }

  return getUnsavedCollectionLabel(input.isSafe, input.labels);
}

export function getCorridorStateName(input: CorridorStateNameInput): string {
  return getCollectionDisplayName({
    name: input.stateName,
    isUnsaved: input.isUnsaved,
    isSafe: input.isSafe,
    labels: input.labels,
  });
}

// The hook lives here so consumers get both the type and the i18n-resolved
// value from one import. useMemo keeps identity stable across renders.
export function useUnsavedLabels(): UnsavedCollectionLabels {
  const { t } = useTranslation('layout');
  return useMemo(
    () => ({
      safeLabel: t('context.unsaved_safe', 'Unsaved SAFE Preset'),
      unsafeLabel: t('context.unsaved_unsafe', 'Unsaved UNSAFE Preset'),
    }),
    [t],
  );
}
