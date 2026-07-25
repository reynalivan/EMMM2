import { expect } from '@wdio/globals';
import { createMockGame, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';

interface GameConfig {
  id: string;
  name: string;
  [key: string]: unknown;
}
interface AppSettings {
  games: GameConfig[];
  [key: string]: unknown;
}

/**
 * Fase 1b — Game management depth (tc-02 edges). Edit/remove happen through
 * save_settings' games array (GamesTab pattern), plus auto-detect. All IPC.
 */
describe('Fase 1b — Game Edit / Remove / Auto-detect', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('Phase1b');
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-02-06: Editing a game name via settings persists', async () => {
    const settings = await invokeInApp<AppSettings>('get_settings');
    const games = settings.games.map((g) =>
      g.id === gameId ? { ...g, name: 'Renamed E2E Game' } : g,
    );
    await invokeInApp('save_settings', { settings: { ...settings, games } });

    const after = await invokeInApp<GameConfig[]>('get_games');
    expect(after.find((g) => g.id === gameId)?.name).toBe('Renamed E2E Game');
  });

  it('TC-02-07: Removing a game drops it from the list', async () => {
    const disposable = await createMockGame('Phase1bDrop');
    try {
      const created = await invokeInApp<GameConfig>('add_game_manual', {
        gameType: 'Genshin',
        path: disposable.root,
      });
      const settings = await invokeInApp<AppSettings>('get_settings');
      const games = settings.games.filter((g) => g.id !== created.id);
      await invokeInApp('save_settings', { settings: { ...settings, games } });

      const after = await invokeInApp<GameConfig[]>('get_games');
      expect(after.some((g) => g.id === created.id)).toBe(false);
    } finally {
      await removeMockGame(disposable);
    }
  });

  it('TC-02-08: Auto-detect returns a list without throwing', async () => {
    const detected = await invokeInApp<GameConfig[]>('auto_detect_games', {});
    expect(Array.isArray(detected)).toBe(true);
  });

  it('TC-02-09: Auto-close launcher setting persists', async () => {
    await invokeInApp('set_auto_close_launcher', { enabled: true });
    expect((await invokeInApp<AppSettings>('get_settings')).auto_close_launcher).toBe(true);
    await invokeInApp('set_auto_close_launcher', { enabled: false });
    expect((await invokeInApp<AppSettings>('get_settings')).auto_close_launcher).toBe(false);
  });
});
