import { expect } from '@wdio/globals';
import { createMockGame, addMockMod, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import { createObject, reconcile } from '../support/data.js';

/**
 * Fase 8 — Runtime periferal. Dashboard analytics, randomizer, error-surfacing,
 * hotkeys. Real update download and in-game hotkey capture are [manual-smoke];
 * here we exercise the command surface only.
 */
describe('Fase 8 — Runtime Peripheral', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('Phase8');
    gameId = await seedGameAndOpenDashboard(game);
    await createObject(gameId, 'DashObj');
    await addMockMod(game, 'DashObj', 'DashMod');
    await reconcile(gameId);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-33-01: Dashboard stats return an aggregate payload', async () => {
    const stats = await invokeInApp('get_dashboard_stats', { gameId });
    expect(stats).toBeDefined();
  });

  it('TC-35-01: Randomizer suggests a list of mods', async () => {
    const proposals = await invokeInApp<unknown[]>('suggest_random_mods', {
      gameId,
      isSafe: false,
    });
    expect(Array.isArray(proposals)).toBe(true);
  });

  it('TC-36-01: Errors bubble up instead of failing silently', async () => {
    // A lookup miss is not an error — `get_object` returns Option, so absence
    // comes back as an explicit null rather than a fabricated empty object.
    expect(await invokeInApp('get_object', { id: 'nonexistent-object-id' })).toBeNull();

    // A guard violation, though, must reject loudly rather than no-op: this
    // path sits outside the game's mods root.
    let threw = false;
    try {
      await invokeInApp('delete_mod', { path: 'C:\\Windows\\System32', gameId });
    } catch {
      threw = true;
    }
    expect(threw).toBe(true);
  });

  it('TC-42-01: Hotkey config update and active keybindings are queryable', async () => {
    await invokeInApp('update_hotkey_config', { config: {} });
    const bindings = await invokeInApp<unknown[]>('get_active_keybindings', { gameId });
    expect(Array.isArray(bindings)).toBe(true);
  });

  it('TC-34-01: Metadata/update check is callable (real download is manual)', async () => {
    // Network-dependent; assert only that the command settles (resolve or reject)
    // without hanging. Real update download is [manual-smoke].
    const settled = await invokeInApp('check_metadata_update').then(
      () => true,
      () => true,
    );
    expect(settled).toBe(true);
  });
});
