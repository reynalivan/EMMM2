import { describe, expect, it, vi } from 'vitest';
import { createInstance } from 'i18next';
import type { HotkeyNotice } from '@/shared/api/tauri/bindings.gen';
import { hotkeyNoticeFixture } from '@/tests/testing/hotkeyNoticeFixture';
import settingsEn from '@/shared/i18n/locales/en/settings.json';
import { mountHotkeyNoticeRenderer } from './renderer';

describe('lightweight native hotkey notice renderer', () => {
  it('renders names as text and acknowledges only a fully rendered owned notice', async () => {
    const translation = createInstance();
    await translation.init({ lng: 'en', resources: { en: { settings: settingsEn } } });
    const root = document.createElement('main');
    const ready = vi.fn().mockResolvedValue(undefined);
    const notice = hotkeyNoticeFixture({ collection: '<img src=x onerror=alert(1)>' });
    let receive!: (notice: HotkeyNotice) => void;
    const unlisten = vi.fn();
    const stop = await mountHotkeyNoticeRenderer(root, translation, {
      listen: async (callback) => {
        receive = callback;
        return unlisten;
      },
      get: async () => notice,
      ready,
    });
    expect(root.textContent).toContain('<img src=x onerror=alert(1)>');
    expect(root.querySelector('img')).toBeNull();
    expect(root.textContent).toContain('Hotkey helpers');
    expect(root.textContent).toContain('Ctrl+F10');
    expect(ready).toHaveBeenCalledWith(notice.id);
    receive(hotkeyNoticeFixture({ sequence: 0, collection: 'Old' }));
    expect(root.textContent).not.toContain('Old');
    stop();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('does not resurrect an expired snapshot or acknowledge it', async () => {
    const translation = createInstance();
    await translation.init({ lng: 'en', resources: { en: { settings: settingsEn } } });
    const root = document.createElement('main');
    const ready = vi.fn();
    const stop = await mountHotkeyNoticeRenderer(root, translation, {
      listen: async () => vi.fn<() => void>(),
      get: async () => hotkeyNoticeFixture({ expires_at_ms: Date.now() - 1 }),
      ready,
    });
    expect(root.childElementCount).toBe(0);
    expect(ready).not.toHaveBeenCalled();
    stop();
  });

  it('retains its listener after a failed initial snapshot so the next notice can recover', async () => {
    const translation = createInstance();
    await translation.init({ lng: 'en', resources: { en: { settings: settingsEn } } });
    const root = document.createElement('main');
    const ready = vi.fn().mockResolvedValue(undefined);
    const unlisten = vi.fn<() => void>();
    let receive!: (notice: HotkeyNotice) => void;
    const log = vi.spyOn(console, 'error').mockImplementation(() => {});
    const stop = await mountHotkeyNoticeRenderer(root, translation, {
      listen: async (callback) => {
        receive = callback;
        return unlisten;
      },
      get: async () => {
        throw new Error('Transient snapshot failure');
      },
      ready,
    });
    expect(unlisten).not.toHaveBeenCalled();
    receive(hotkeyNoticeFixture());
    await vi.waitFor(() => expect(ready).toHaveBeenCalledOnce());
    expect(root.textContent).toContain('Combat');
    expect(log).toHaveBeenCalledOnce();
    stop();
    expect(unlisten).toHaveBeenCalledOnce();
    log.mockRestore();
  });
});
