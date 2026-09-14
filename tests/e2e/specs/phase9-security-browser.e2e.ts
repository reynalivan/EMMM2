import { expect } from '@wdio/globals';
import { createMockGame, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import { createObject } from '../support/data.js';
import type { GameObject } from '../../../src/shared/api/tauri/bindings.gen.js';

/**
 * Fase 9 — Keamanan, tema, browser (beyond Fase 8).
 * PIN authentication remains a manual smoke flow because a stale PIN can lock
 * the app on the next boot. Object pinning is safe to round-trip in the
 * isolated E2E database and is covered below instead.
 */
describe('Fase 9 — Security, Themes & Browser', () => {
  let game: MockGame;
  let objectId: string;

  before(async () => {
    game = await createMockGame('Phase9');
    const gameId = await seedGameAndOpenDashboard(game);
    objectId = await createObject(gameId, 'PinnedObject');
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-45-01: Object pin state round-trips through the current command', async () => {
    const before = await invokeInApp<GameObject | null>('get_object', { id: objectId });
    expect(before?.is_pinned).toBe(false);

    await invokeInApp('pin_object', { id: objectId, pin: true });
    const pinned = await invokeInApp<GameObject | null>('get_object', { id: objectId });
    expect(pinned?.is_pinned).toBe(true);

    await invokeInApp('pin_object', { id: objectId, pin: false });
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
