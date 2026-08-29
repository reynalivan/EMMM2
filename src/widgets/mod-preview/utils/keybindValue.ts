const MODIFIER_TOKENS = ['ctrl', 'alt', 'shift', 'no_ctrl', 'no_alt', 'no_shift'];

export interface KeybindModifierState {
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  noCtrl: boolean;
  noAlt: boolean;
  noShift: boolean;
}

export interface ParsedKeybindValue extends KeybindModifierState {
  mainKey: string | null;
}

export function parseKeybindValue(value: string): ParsedKeybindValue {
  const lower = value.toLowerCase();
  const parts = value.split(/\s+/).filter((part) => !MODIFIER_TOKENS.includes(part.toLowerCase()));

  return {
    ctrlKey: lower.includes('ctrl') && !lower.includes('no_ctrl'),
    altKey: lower.includes('alt') && !lower.includes('no_alt'),
    shiftKey: lower.includes('shift') && !lower.includes('no_shift'),
    noCtrl: lower.includes('no_ctrl'),
    noAlt: lower.includes('no_alt'),
    noShift: lower.includes('no_shift'),
    mainKey: parts.length > 0 ? parts[parts.length - 1].toUpperCase() : null,
  };
}

export function formatKeybindValue(state: KeybindModifierState, mainKey: string): string {
  const parts: string[] = [];

  if (state.ctrlKey) parts.push('ctrl');
  if (state.noCtrl && !state.ctrlKey) parts.push('no_ctrl');

  if (state.altKey) parts.push('alt');
  if (state.noAlt && !state.altKey) parts.push('no_alt');

  if (state.shiftKey) parts.push('shift');
  if (state.noShift && !state.shiftKey) parts.push('no_shift');

  parts.push(mainKey);
  return parts.join(' ');
}
