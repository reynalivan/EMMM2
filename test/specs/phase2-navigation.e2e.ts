import { expect } from '@wdio/globals';
import path from 'path';
import { createMockGame, addMockMod, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import { createObject, reconcile, getObjects, findObject } from '../support/data.js';

interface GameConfig {
  id: string;
  game_type: number;
  [key: string]: unknown;
}
interface CategoryCount {
  [key: string]: unknown;
}
interface FolderEntry {
  name: string;
  is_dir: boolean;
}

/**
 * Fase 2 — Navigasi & permukaan baca. Read-only projection surface: object
 * list, category counts, folder listing, schema/master DB. No disk mutation
 * beyond seeding the fixture library.
 */
describe('Fase 2 — Navigation & Read Surface', () => {
  let game: MockGame;
  let gameId: string;
  let gameType: number;

  before(async () => {
    game = await createMockGame('Phase2');
    gameId = await seedGameAndOpenDashboard(game);
    const games = await invokeInApp<GameConfig[]>('get_games');
    gameType = games.find((g) => g.id === gameId)!.game_type;

    // Seed a small library: two objects, one with two mods.
    await createObject(gameId, 'Raiden');
    await createObject(gameId, 'Nahida');
    await addMockMod(game, 'Raiden', 'SkinA');
    await addMockMod(game, 'Raiden', 'SkinB');
    await reconcile(gameId);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-07-01: Object list returns seeded objects with mod counts', async () => {
    const objects = await getObjects(gameId);
    const names = objects.map((o) => o.name);
    expect(names).toContain('Raiden');
    expect(names).toContain('Nahida');

    const raiden = objects.find((o) => o.name === 'Raiden')!;
    expect(raiden.mod_count).toBeGreaterThanOrEqual(2);
    expect(raiden.enabled_count).toBeGreaterThanOrEqual(2);
  });

  it('TC-08-01: Category counts command returns a list without error', async () => {
    const counts = await invokeInApp<CategoryCount[]>('get_category_counts_cmd', { gameId });
    expect(Array.isArray(counts)).toBe(true);
  });

  it('TC-11-01: Folder listing classifies mod folders under an object', async () => {
    const raiden = await findObject(gameId, 'Raiden');
    expect(raiden).toBeDefined();
    // `folder_path` is a stored key, deliberately not usable as a filesystem
    // path — the command wants a real absolute path under the mods dir.
    const entries = await invokeInApp<FolderEntry[]>('list_folder_entries_cmd', {
      folderPath: path.join(game.modsPath, 'Raiden'),
      gameId,
    });
    const dirNames = entries.filter((e) => e.is_dir).map((e) => e.name);
    expect(dirNames).toContain('SkinA');
    expect(dirNames).toContain('SkinB');
  });

  it('TC-09-01: Game schema and master DB load for the game type', async () => {
    const schema = await invokeInApp('get_game_schema', { gameType });
    expect(schema).toBeDefined();

    const masterDb = await invokeInApp<string>('get_master_db', { gameType });
    expect(typeof masterDb).toBe('string');
    expect(masterDb.length).toBeGreaterThan(0);
  });
});
