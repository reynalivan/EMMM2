import { describe, expect, it } from 'vitest';
import type { TFunction } from 'i18next';
import type { HotkeyConfig } from '../../../types/settings';
import { detectConflicts } from './hotkeyConflicts';

const translate = ((key: string, values?: Record<string, unknown>) =>
  `${key}:${JSON.stringify(values ?? {})}`) as TFunction;

const defaults: HotkeyConfig = {
  enabled: true,
  cooldown_ms: 500,
  next_preset: 'Ctrl+F6',
  prev_preset: 'Shift+F6',
  toggle_overlay: 'F7',
  next_variant: 'Ctrl+F8',
  prev_variant: 'Shift+F8',
};

describe('hotkey conflict detection', () => {
  it('keeps the modifier-based defaults clear of package keys', () => {
    expect(
      detectConflicts(
        defaults,
        [
          { label: 'package', key: 'F6' },
          { label: 'frame analysis', key: 'F8' },
          { label: 'reload', key: 'F10' },
        ],
        translate,
      ),
    ).toEqual([]);
  });

  it('reports a runtime-reserved collision', () => {
    const conflicts = detectConflicts(
      { ...defaults, next_preset: 'f10' },
      [{ label: 'reload', key: 'F10' }],
      translate,
    );

    expect(conflicts).toHaveLength(1);
    expect(conflicts[0]).toContain('reserved_message');
  });
});
