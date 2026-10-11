import { describe, expect, it } from 'vitest';
import { createInstance } from 'i18next';
import type { HotkeyNotice } from '@/shared/api/tauri/bindings.gen';
import { hotkeyNoticeFixture } from '@/tests/testing/hotkeyNoticeFixture';
import settingsEn from '@/shared/i18n/locales/en/settings.json';
import { acceptHotkeyNotice, buildHotkeyNoticeView } from './hotkeyNotice';

const now = Date.UTC(2026, 9, 11, 12);

const noticeFixture = (patch: Partial<HotkeyNotice> = {}) =>
  hotkeyNoticeFixture({
    created_at_ms: now,
    expires_at_ms: now + 5000,
    ...patch,
  });

const translation = createInstance();
await translation.init({ lng: 'en', resources: { en: { settings: settingsEn } } });
const t = translation.getFixedT('en', 'settings');

describe('hotkey notice ownership and helper content', () => {
  it('rejects foreign games, changed roots, expired and invalid-time notices', () => {
    const context = { gameId: 'game-a', modsPath: 'C:\\fixture\\Mods', now };
    expect(acceptHotkeyNotice(null, noticeFixture(), context)).toBe(true);
    for (const patch of [
      { game_id: 'game-b' },
      { mods_path: 'C:\\replacement\\Mods' },
      { expires_at_ms: now },
      { created_at_ms: now + 10_000 },
      { expires_at_ms: Number.NaN },
      { sequence: Number.POSITIVE_INFINITY },
    ]) {
      expect(acceptHotkeyNotice(null, noticeFixture(patch), context)).toBe(false);
    }
  });

  it('does not let an older snapshot or duplicate replace the newest event', () => {
    const current = noticeFixture({ id: 'latest', sequence: 3 });
    const context = { gameId: 'game-a', modsPath: current.mods_path, now };
    expect(acceptHotkeyNotice(current, current, context)).toBe(false);
    expect(acceptHotkeyNotice(current, noticeFixture({ sequence: 2 }), context)).toBe(false);
    expect(acceptHotkeyNotice(current, noticeFixture({ sequence: 4 }), context)).toBe(true);
  });

  it('accepts only forward runtime enrichment of the same visible receipt', () => {
    const current = noticeFixture();
    const terminal = noticeFixture({
      runtime_status: {
        game_id: current.game_id,
        generation: 2,
        phase: 'succeeded',
        cause: 'collection_applied',
        message: null,
        reload_status: 'sent',
        reload_binding: 'Ctrl+F10',
      },
    });
    const context = { gameId: current.game_id, modsPath: current.mods_path, now };
    expect(acceptHotkeyNotice(current, terminal, context)).toBe(true);
    expect(acceptHotkeyNotice(terminal, current, context)).toBe(false);
    expect(acceptHotkeyNotice(terminal, terminal, context)).toBe(false);
  });

  it('distinguishes sent, unnecessary and manual reload without claiming rendering', () => {
    const status = {
      game_id: 'game-a',
      generation: 2,
      phase: 'succeeded' as const,
      cause: 'collection_applied',
      message: null,
      reload_status: 'sent' as const,
      reload_binding: 'Ctrl+F10',
    };
    const sent = buildHotkeyNoticeView(noticeFixture(), t, undefined, status);
    expect(sent.reload).toBe('Reload command sent: Ctrl+F10.');
    expect(sent.detail).toBe('Runtime update completed. Game rendering is not confirmed.');
    const notRequired = buildHotkeyNoticeView(noticeFixture(), t, undefined, {
      ...status,
      reload_status: 'not_required',
    });
    expect(notRequired.reload).toBe('No automatic reload is required for this update.');
    const manual = buildHotkeyNoticeView(noticeFixture(), t, undefined, {
      ...status,
      phase: 'needs_manual_reload',
      reload_status: 'manual',
    });
    expect(manual.reload).toContain('Press Ctrl+F10');
  });

  it('retains the discovered reload binding while terminal status is still pending', () => {
    const pending = {
      game_id: 'game-a',
      generation: 2,
      phase: 'running' as const,
      cause: 'collection_applied',
      message: null,
      reload_status: null,
      reload_binding: null,
    };
    const view = buildHotkeyNoticeView(noticeFixture(), t, undefined, pending);
    expect(view.reload).toContain('Press Ctrl+F10');
    expect(view.hints[4].binding).toBe('Ctrl+F10');
    const manualUnknown = buildHotkeyNoticeView(noticeFixture(), t, undefined, {
      ...pending,
      phase: 'needs_manual_reload',
      reload_status: 'manual',
    });
    expect(manualUnknown.hints[4].enabled).toBe(false);
  });

  it('does not let delayed global running state override embedded terminal proof', () => {
    const terminal = {
      game_id: 'game-a',
      generation: 2,
      phase: 'succeeded' as const,
      cause: 'collection_applied',
      message: null,
      reload_status: 'sent' as const,
      reload_binding: 'Ctrl+F10',
    };
    const notice = noticeFixture({ runtime_status: terminal });
    const view = buildHotkeyNoticeView(notice, t, undefined, {
      ...terminal,
      phase: 'running',
      reload_status: null,
      reload_binding: null,
    });
    expect(view.reload).toBe('Reload command sent: Ctrl+F10.');
    expect(view.detail).toBe('Runtime update completed. Game rendering is not confirmed.');
    const lateTerminal = buildHotkeyNoticeView(
      noticeFixture({ runtime_status: { ...terminal, phase: 'running' } }),
      t,
      undefined,
      terminal,
    );
    expect(lateTerminal.reload).toBe(view.reload);
  });

  it('highlights the new collection and includes every actual control binding', () => {
    const view = buildHotkeyNoticeView(noticeFixture(), t);
    expect(view.primary).toBe('Combat');
    expect(view.origin).toBe('From Exploration');
    expect(view.reload).toContain('Ctrl+F10');
    expect(view.hints.map((hint) => hint.binding)).toEqual([
      'Alt+F5',
      'Shift+F5',
      'Ctrl+F5',
      'F7',
      'Ctrl+F10',
    ]);
  });

  it('does not claim a switch for one collection or game rendering after commit', () => {
    const view = buildHotkeyNoticeView(noticeFixture({ previous_collection: 'Combat' }), t);
    expect(view.title).toBe('Collection applied');
    expect(view.detail).toBe('Mod changes saved. Runtime update pending.');
  });

  it('marks disabled shortcuts independently from KeyViewer and unknown reload', () => {
    const notice = noticeFixture({ reload_binding: null, keyviewer_enabled: true });
    const view = buildHotkeyNoticeView(
      { ...notice, hotkeys: { ...notice.hotkeys, enabled: false } },
      t,
    );
    expect(view.hints.slice(0, 3).every((hint) => !hint.enabled)).toBe(true);
    expect(view.hints[3].enabled).toBe(true);
    expect(view.hints[4].enabled).toBe(false);
    expect(view.reload).toBe('Reload the configuration in game; its binding is unavailable.');
  });

  it.each(['safe_mode_changed', 'no_presets', 'failed'] as const)(
    'retains helpers without false collection success for %s',
    (kind) => {
      const view = buildHotkeyNoticeView(
        noticeFixture({ kind, safe_mode: true, message: 'Locked folder' }),
        t,
      );
      expect(view.hints).toHaveLength(5);
      expect(view.title).not.toBe('Switched collection');
      if (kind === 'safe_mode_changed') expect(view.primary).toBe('Safe Mode ON');
      if (kind === 'failed') expect(view.detail).toBe('Locked folder');
    },
  );
});
