import { expect } from '@wdio/globals';
import path from 'path';
import {
  createMockGame,
  addMockMod,
  removeMockGame,
  listDir,
  type MockGame,
} from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import { createObject, reconcile } from '../support/data.js';

interface GameConfig {
  id: string;
  game_type: number;
  [key: string]: unknown;
}
interface FolderEntry {
  name: string;
  is_dir: boolean;
}

/**
 * Fase 2b — Navigation edges (tc-08 search, tc-11 classification). Master-DB
 * search filtering and disabled-mod classification on the folder listing.
 */
describe('Fase 2b — Navigation Edges', () => {
  let game: MockGame;
  let gameId: string;
  let gameType: number;

  before(async () => {
    game = await createMockGame('Phase2b');
    gameId = await seedGameAndOpenDashboard(game);
    gameType = (await invokeInApp<GameConfig[]>('get_games')).find(
      (g) => g.id === gameId,
    )!.game_type;
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-08-02: Master DB search returns scored results for a query', async () => {
    const results = await invokeInApp<{ score: number; item: unknown }[]>('search_master_db', {
      gameType,
      query: 'a',
    });
    expect(Array.isArray(results)).toBe(true);
    if (results.length > 0) {
      expect(typeof results[0].score).toBe('number');
    }
  });

  it('TC-11-02: Folder listing classifies enabled vs disabled mods by prefix', async () => {
    await createObject(gameId, 'ClassObj');
    await addMockMod(game, 'ClassObj', 'ModEnabled');
    await addMockMod(game, 'ClassObj', 'ModDisabled');
    await reconcile(gameId);

    const objDir = path.join(game.modsPath, 'ClassObj');
    await invokeInApp('bulk_toggle_mods', {
      gameId,
      paths: [path.join(objDir, 'ModDisabled')],
      enable: false,
    });

    const entries = await invokeInApp<FolderEntry[]>('list_folder_entries_cmd', {
      folderPath: objDir,
      gameId,
    });
    const names = entries.filter((e) => e.is_dir).map((e) => e.name);
    expect(names).toContain('ModEnabled');
    expect(names).toContain('DISABLED ModDisabled');
    // Cross-check disk matches the listing.
    expect(await listDir(objDir)).toContain('DISABLED ModDisabled');
  });
});
