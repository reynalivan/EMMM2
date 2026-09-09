import { expect } from '@wdio/globals';
import { createMockGame, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import type { AppSettings, GameConfig } from '../../src/shared/api/tauri/bindings.gen.js';

/**
 * Fase 1 — Gerbang masuk (Settings + Game Management).
 * Onboarding (tc-03) and manual-add-via-form (tc-02) already have dedicated
 * specs; this covers settings persistence and the multi-game IPC surface.
 */
describe('Fase 1 — Gates (Settings & Game Management)', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('Phase1');
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-04-01: Theme + language changes persist through save/reload', async () => {
    const before = await invokeInApp<AppSettings>('get_settings');
    const nextTheme = before.theme === 'dark' ? 'light' : 'dark';

    await invokeInApp('save_settings', {
      settings: { ...before, theme: nextTheme, language: 'id' },
    });

    const after = await invokeInApp<AppSettings>('get_settings');
    expect(after.theme).toBe(nextTheme);
    expect(after.language).toBe('id');
  });

  it('TC-04-02: Safety keyword persist', async () => {
    const before = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', {
      settings: {
        ...before,
        safety: { ...before.safety, keywords: ['nsfw'] },
      },
    });

    const after = await invokeInApp<AppSettings>('get_settings');
    expect(after.safety.keywords).toContain('nsfw');

    // Restore the complete snapshot so this spec does not leak settings.
    await invokeInApp('save_settings', {
      settings: { ...after, safety: before.safety },
    });
  });

  it('TC-02-01: Games list reflects seeded game and supports add + switch', async () => {
    const initial = await invokeInApp<GameConfig[]>('get_games');
    expect(initial.some((g) => g.id === gameId)).toBe(true);

    const second = await createMockGame('Phase1b');
    try {
      const created = await invokeInApp<GameConfig>('add_game_manual', {
        gameType: 'GIMI',
        path: second.root,
      });
      // add_game_manual only validates and returns a candidate; this is what stores it.
      await invokeInApp('save_onboarding_games', { games: [created] });
      const grown = await invokeInApp<GameConfig[]>('get_games');
      expect(grown.length).toBeGreaterThan(initial.length);

      // Switch active back and forth — no throw, state consistent.
      await invokeInApp('set_active_game', { gameId: created.id });
      await invokeInApp('set_active_game', { gameId });
    } finally {
      await removeMockGame(second);
    }
  });

  // launch_game spawns the real loader process → covered as [manual-smoke].
});
