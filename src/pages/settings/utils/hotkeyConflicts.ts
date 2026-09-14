import type { TFunction } from 'i18next';
import type { HotkeyConfig } from '@/entities/settings';

export interface ReservedBinding {
  label: string;
  key: string;
}

/** Detect local duplicates and collisions with known runtime bindings. */
export function detectConflicts(
  config: HotkeyConfig,
  reserved: ReservedBinding[],
  t: TFunction,
): string[] {
  const bindings: [string, string][] = [
    [t('settings:hotkeys.labels.safe_mode'), config.safe_mode ?? ''],
    [t('settings:hotkeys.labels.next_preset'), config.next_preset ?? ''],
    [t('settings:hotkeys.labels.prev_preset'), config.prev_preset ?? ''],
    [t('settings:hotkeys.labels.toggle_overlay'), config.toggle_overlay ?? ''],
  ];

  const conflicts: string[] = [];
  for (let i = 0; i < bindings.length; i++) {
    for (let j = i + 1; j < bindings.length; j++) {
      if (bindings[i][1].toLowerCase() === bindings[j][1].toLowerCase()) {
        conflicts.push(
          t('settings:hotkeys.conflicts.message', {
            label1: bindings[i][0],
            label2: bindings[j][0],
            key: bindings[i][1],
          }),
        );
      }
    }
  }
  for (const [label, key] of bindings) {
    for (const runtime of reserved) {
      if (key.trim() && key.trim().toLowerCase() === runtime.key.trim().toLowerCase()) {
        conflicts.push(
          t('settings:hotkeys.conflicts.reserved_message', {
            label,
            runtime: runtime.label,
            key,
          }),
        );
      }
    }
  }
  return conflicts;
}
