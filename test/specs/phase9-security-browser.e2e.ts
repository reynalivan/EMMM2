import { expect } from '@wdio/globals';
import { createMockGame, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';

interface PinStatus {
  has_pin: boolean;
  is_locked: boolean;
  [key: string]: unknown;
}

/**
 * Fase 9 — Keamanan, tema, browser (beyond Fase 8).
 * NOTE: set_pin is intentionally NOT automated — a stale PIN can lock the app
 * on next boot (boot guard), and this suite writes to the real app_data DB.
 * The set/verify/reset round-trip is [manual-smoke]. Here PIN is read-only.
 */
describe('Fase 9 — Security (read-only), Themes & Browser', () => {
  let game: MockGame;

  before(async () => {
    game = await createMockGame('Phase9');
    await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-45-01: PIN status is queryable (read-only)', async () => {
    const status = await invokeInApp<PinStatus>('get_pin_status');
    expect(typeof status.has_pin).toBe('boolean');
    expect(typeof status.is_locked).toBe('boolean');
  });

  it('TC-46-01: Custom themes list is queryable', async () => {
    const themes = await invokeInApp<unknown[]>('list_custom_themes');
    expect(Array.isArray(themes)).toBe(true);
  });

  it('TC-47-01: Browser homepage round-trips and downloads list is queryable', async () => {
    const original = await invokeInApp<string>('browser_get_homepage');

    await invokeInApp('browser_set_homepage', { url: 'https://gamebanana.com/' });
    const updated = await invokeInApp<string>('browser_get_homepage');
    expect(updated).toBe('https://gamebanana.com/');

    // Restore original homepage.
    await invokeInApp('browser_set_homepage', { url: original });

    const downloads = await invokeInApp<unknown[]>('browser_list_downloads');
    expect(Array.isArray(downloads)).toBe(true);
  });
});
