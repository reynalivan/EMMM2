import { expect } from '@wdio/globals';
import { createMockGame, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';

interface AppSettings {
  keyviewer: Record<string, unknown>;
  hotkeys: Record<string, unknown>;
  language: string;
  [key: string]: unknown;
}

/**
 * Fase 1c — Settings depth (tc-04 edges). Maintenance, thumbnail cleanup,
 * hotkey config, and per-field persistence beyond theme/language.
 */
describe('Fase 1c — Settings Depth', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('Phase1c');
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-04-10: Run maintenance completes and returns a summary', async () => {
    const result = await invokeInApp<string>('run_maintenance', { gameId });
    expect(typeof result).toBe('string');
  });

  it('TC-04-11: Clear old thumbnails completes', async () => {
    const result = await invokeInApp<string>('clear_old_thumbnails');
    expect(typeof result).toBe('string');
  });

  it('TC-04-12: Hotkey config update is accepted', async () => {
    await invokeInApp('update_hotkey_config', { config: { toggle_overlay: 'F8' } });
    const settings = await invokeInApp<AppSettings>('get_settings');
    expect(settings.hotkeys).toBeDefined();
  });

  it('TC-04-13: Keyviewer config persists through save/reload', async () => {
    const before = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', {
      settings: { ...before, keyviewer: { ...before.keyviewer, enabled: true } },
    });
    const after = await invokeInApp<AppSettings>('get_settings');
    expect(after.keyviewer.enabled).toBe(true);

    await invokeInApp('save_settings', {
      settings: { ...after, keyviewer: { ...after.keyviewer, enabled: false } },
    });
  });
});
