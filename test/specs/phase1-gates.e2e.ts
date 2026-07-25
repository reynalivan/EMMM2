import { expect } from '@wdio/globals';
import { createMockGame, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';

/** Minimal AppSettings subset this phase mutates. */
interface AppSettings {
  theme: string;
  language: string;
  safe_mode: { enabled?: boolean; keywords: string[] };
  [key: string]: unknown;
}
interface GameConfig {
  id: string;
  [key: string]: unknown;
}

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

  it('TC-04-02: Safe-mode toggle + keyword persist', async () => {
    const before = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', {
      settings: {
        ...before,
        safe_mode: { ...before.safe_mode, enabled: true, keywords: ['nsfw'] },
      },
    });

    const after = await invokeInApp<AppSettings>('get_settings');
    expect(after.safe_mode.enabled).toBe(true);
    expect(after.safe_mode.keywords).toContain('nsfw');

    // Reset so later phases run in normal mode.
    await invokeInApp('save_settings', {
      settings: { ...after, safe_mode: { ...after.safe_mode, enabled: false } },
    });
  });

  it('TC-02-01: Games list reflects seeded game and supports add + switch', async () => {
    const initial = await invokeInApp<GameConfig[]>('get_games');
    expect(initial.some((g) => g.id === gameId)).toBe(true);

    const second = await createMockGame('Phase1b');
    try {
      const created = await invokeInApp<GameConfig>('add_game_manual', {
        gameType: 'Genshin',
        path: second.root,
      });
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
