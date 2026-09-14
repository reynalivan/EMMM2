import { expect } from '@wdio/globals';
import { createMockGame, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import type { AppSettings } from '../../../src/shared/api/tauri/bindings.gen.js';

/**
 * Fase 1c — Settings depth (tc-04 edges). Maintenance, thumbnail cleanup,
 * hotkey config, and per-field persistence beyond theme/language.
 */
describe('Fase 1c — Settings Depth', () => {
  let game: MockGame;

  before(async () => {
    game = await createMockGame('Phase1c');
    await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-04-10: Run maintenance completes and returns the reclaimed count', async () => {
    const result = await invokeInApp<number>('run_maintenance');
    expect(typeof result).toBe('number');
  });

  it('TC-04-11: Clear old thumbnails completes', async () => {
    const removed = await invokeInApp<number>('clear_old_thumbnails');
    expect(typeof removed).toBe('number');
  });

  it('TC-04-12: Hotkey config update is accepted', async () => {
    const before = await invokeInApp<AppSettings>('get_settings');
    if (!before.hotkeys) throw new Error('E2E settings did not include hotkey configuration');

    // Global shortcuts belong to the OS, so the E2E process must not compete
    // with a developer's running application for the shipped default keys.
    await invokeInApp('save_settings', {
      settings: { ...before, hotkeys: { ...before.hotkeys, enabled: false } },
    });
    await invokeInApp('update_hotkey_config');
    const settings = await invokeInApp<AppSettings>('get_settings');
    expect(settings.hotkeys?.enabled).toBe(false);
  });

  it('TC-04-13: Keyviewer config persists through save/reload', async () => {
    const before = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', {
      settings: {
        ...before,
        keyviewer: { ...(before.keyviewer ?? { enabled: false }), enabled: true },
      },
    });
    const after = await invokeInApp<AppSettings>('get_settings');
    expect(after.keyviewer?.enabled).toBe(true);

    await invokeInApp('save_settings', {
      settings: {
        ...after,
        keyviewer: { ...(after.keyviewer ?? { enabled: true }), enabled: false },
      },
    });
  });
});
