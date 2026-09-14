import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { TFunction } from 'i18next';
import { createElement } from 'react';
import { useQuery, type UseQueryResult } from '@tanstack/react-query';
import type { HotkeyConfig } from '@/entities/settings';
import type { KeyViewerRuntimeDiagnostics } from '@/shared/api/tauri/bindings';
import { detectConflicts } from '../../utils/hotkeyConflicts';
import { render, screen } from '../../../../tests/testing/test-utils';
import HotkeyTab from './HotkeyTab';

const mockSaveSettingsAsync = vi.fn();
const mockGetReloadKey = vi.fn();
let activeGameId: string | null = null;

const inactiveSettings = {
  active_game_id: null,
  hotkeys: {
    enabled: true,
    safe_mode: 'F5',
    next_preset: 'Ctrl+F6',
    prev_preset: 'Shift+F6',
    toggle_overlay: 'F7',
  },
  keyviewer: { enabled: true },
};

const activeSettings = { ...inactiveSettings, active_game_id: 'gimi' };

vi.mock('@/entities/settings', () => ({
  useSettings: () => ({
    settings: activeGameId === 'gimi' ? activeSettings : inactiveSettings,
    saveHotkeyConfiguration: mockSaveSettingsAsync,
  }),
}));

vi.mock('../../../../shared/api/tauri/bindings', () => ({
  commands: {
    getReloadKey: (...args: unknown[]) => mockGetReloadKey(...args),
    saveHotkeyConfiguration: vi.fn(),
  },
}));

vi.mock('@/shared/ui/toast', () => ({
  useToastStore: () => ({ addToast: vi.fn() }),
}));

const translate = ((key: string, values?: Record<string, unknown>) =>
  `${key}:${JSON.stringify(values ?? {})}`) as TFunction;

const defaults: HotkeyConfig = {
  enabled: true,
  safe_mode: 'F5',
  next_preset: 'Ctrl+F6',
  prev_preset: 'Shift+F6',
  toggle_overlay: 'F7',
};

describe('hotkey conflict detection', () => {
  beforeEach(() => {
    activeGameId = null;
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

  it('shows only the four supported controls', () => {
    render(createElement(HotkeyTab));

    expect(screen.getByLabelText('Safe Mode')).toBeInTheDocument();
    expect(screen.getByLabelText('Next preset')).toBeInTheDocument();
    expect(screen.getByLabelText('Previous preset')).toBeInTheDocument();
    expect(screen.getByLabelText('Toggle overlay')).toBeInTheDocument();
    expect(screen.queryByText(/next_variant/)).toBeNull();
    expect(screen.queryByText(/prev_variant/)).toBeNull();
  });

  it('shows concise KeyViewer diagnostics and disables cleanup without a game executable', () => {
    activeGameId = 'gimi';
    vi.mocked(useQuery).mockReturnValue({
      data: {
        last_sync_unix_ms: Date.UTC(2026, 8, 14, 8, 30),
        publication: 'published',
        reload: 'manual',
        reload_binding: 'Ctrl+F10',
        cleanup_automatic_disabled: true,
      } satisfies KeyViewerRuntimeDiagnostics,
      isError: false,
      isLoading: false,
      refetch: vi.fn(),
    } as unknown as UseQueryResult<KeyViewerRuntimeDiagnostics>);

    render(createElement(HotkeyTab));

    expect(screen.getByText('KeyViewer runtime')).toBeInTheDocument();
    expect(screen.getByText('Published')).toBeInTheDocument();
    expect(screen.getByText('Manual: Ctrl+F10')).toBeInTheDocument();
    expect(screen.getByText('Waiting for a configured game executable')).toBeInTheDocument();
    expect(
      screen.getByText(
        'EMMM keeps temporary recovery files until it can verify the game is stopped or the new overlay has reloaded.',
      ),
    ).toBeInTheDocument();
  });
});
