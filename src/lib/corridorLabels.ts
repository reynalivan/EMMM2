export const UNSAVED_PRESET_LABEL = 'Unsaved Preset';
export const ALL_DISABLED_LABEL = 'All Disabled';
export const SAFE_MODE_LABEL = 'SAFE';
export const UNSAFE_MODE_LABEL = 'UNSAFE';

export function getCorridorStateName(stateName: string | null | undefined): string {
  return stateName ?? UNSAVED_PRESET_LABEL;
}

export function buildCorridorEmptyStateLabel(stateName: string | null | undefined): string {
  return `${getCorridorStateName(stateName)} is empty (${ALL_DISABLED_LABEL}).`;
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
