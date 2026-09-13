import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { TFunction } from 'i18next';
import { createElement } from 'react';
import type { HotkeyConfig } from '@/entities/settings';
import { detectConflicts } from '../../utils/hotkeyConflicts';
import { render, screen } from '../../../../tests/testing/test-utils';
import HotkeyTab from './HotkeyTab';

const mockSaveSettingsAsync = vi.fn();
const mockGetReloadKey = vi.fn();

vi.mock('@/entities/settings', () => ({
  useSettings: () => ({
    settings: {
      active_game_id: null,
      hotkeys: {
        enabled: true,
        cooldown_ms: 500,
        next_preset: 'Ctrl+F6',
        prev_preset: 'Shift+F6',
        toggle_overlay: 'F7',
        next_variant: 'Ctrl+F8',
        prev_variant: 'Shift+F8',
      },
      keyviewer: { enabled: true },
    },
    saveSettingsAsync: mockSaveSettingsAsync,
  }),
}));

vi.mock('../../../../shared/api/tauri/bindings', () => ({
  commands: {
    getReloadKey: (...args: unknown[]) => mockGetReloadKey(...args),
    updateHotkeyConfig: vi.fn(),
  },
}));

vi.mock('@/shared/ui/toast', () => ({
  useToastStore: () => ({ addToast: vi.fn() }),
}));

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
  beforeEach(() => {
    mockGetReloadKey.mockResolvedValue(null);
  });

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

  it('uses readable info text for the KeyViewer runtime infrastructure panel', () => {
    render(createElement(HotkeyTab));

    const infrastructureTitle = screen.getByText('Runtime files');
    expect(infrastructureTitle.closest('.alert')).toBeNull();
  });
});
