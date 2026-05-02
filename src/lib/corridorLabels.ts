// Two hooks are co-located here so that consumers only need one import for both
// the label *shape* and the *hook* that produces it via i18n.
// Import path: '../../lib/corridorLabels' (relative from features) or '../lib/corridorLabels'
import { useTranslation } from 'react-i18next';
import { useMemo } from 'react';

export const UNSAVED_SAFE_PRESET_LABEL = 'Unsaved SAFE Preset';
export const UNSAVED_UNSAFE_PRESET_LABEL = 'Unsaved UNSAFE Preset';
export const ALL_DISABLED_LABEL = 'All Disabled';
export const SAFE_MODE_LABEL = 'SAFE';
export const UNSAFE_MODE_LABEL = 'UNSAFE';

export type UnsavedCollectionLabels = {
  safeLabel: string;
  unsafeLabel: string;
};

export type CorridorSwitchTextLabels = UnsavedCollectionLabels & {
  allDisabledLabel: string;
  systemFallbackLabel: string;
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

export function buildCorridorEmptyStateLabel(input: CorridorStateNameInput): string {
  return `${getCorridorStateName(input)} is empty (${ALL_DISABLED_LABEL}).`;
}

export function getCorridorSwitchTargetName(input: {
  targetStateKind: string | null | undefined;
  stateName: string | null | undefined;
  isUnsaved: boolean | null | undefined;
  isSafe: boolean | null | undefined;
  labels: CorridorSwitchTextLabels;
}): string {
  if (!input.targetStateKind || input.targetStateKind === 'none') {
    return input.labels.allDisabledLabel;
  }

  if (input.targetStateKind === 'system_fallback') {
    return input.labels.systemFallbackLabel;
  }

  return getCorridorStateName({
    stateName: input.stateName,
    isUnsaved: input.isUnsaved,
    isSafe: input.isSafe,
    labels: input.labels,
  });
}

export function getCorridorModeLabel(isSafeMode: boolean): string {
  return isSafeMode ? SAFE_MODE_LABEL : UNSAFE_MODE_LABEL;
}

export function buildCorridorModeSwitchTitle(targetSafeMode: boolean): string {
  return `Switch to ${getCorridorModeLabel(targetSafeMode)}`;
}

export function buildLeavingCorridorLabel(targetSafeMode: boolean): string {
  return `Current ${getCorridorModeLabel(!targetSafeMode)} State`;
}

export function buildTargetCorridorLabel(targetSafeMode: boolean): string {
  return `Destination ${getCorridorModeLabel(targetSafeMode)} State`;
}

export function buildTargetCorridorDescription(stateName: string | null | undefined): string {
  return stateName ? 'Last Active Collection' : 'No remembered active collection';
}

export function buildMissingTargetCorridorDescription(): string {
  return 'No saved target state. All mods will remain disabled.';
}

// ── React Hooks ─────────────────────────────────────────────────────────────
// These hooks live here so consumers get both the type and the i18n-resolved
// value from a single import. useMemo ensures stable identity across renders.

/**
 * Returns i18n-resolved UnsavedCollectionLabels.
 * Replaces the inline { safeLabel, unsafeLabel } objects in:
 *   CollectionList, CollectionPreviewPanel, CollectionsPage, ApplyCollectionModal
 */
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

/**
 * Returns i18n-resolved CorridorSwitchTextLabels — UnsavedCollectionLabels
 * extended with the two extra keys needed by ModeSwitchConfirmModal.
 */
export function useCorridorSwitchLabels(): CorridorSwitchTextLabels {
  const { t } = useTranslation(['layout', 'safe_mode']);
  return useMemo(
    () => ({
      safeLabel: t('layout:context.unsaved_safe', 'Unsaved SAFE Preset'),
      unsafeLabel: t('layout:context.unsaved_unsafe', 'Unsaved UNSAFE Preset'),
      allDisabledLabel: t('safe_mode:switch.all_disabled', 'All Disabled'),
      systemFallbackLabel: t('safe_mode:switch.restore', 'Restore'),
    }),
    [t],
  );
}
