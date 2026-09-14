import { expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import { createMockGame, addMockMod, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import { createObject, reconcile } from '../support/data.js';

interface ModInfo {
  tags?: string[] | null;
  [key: string]: unknown;
}

/**
 * Fase 4b — Editor edges (tc-17 tags, tc-18 multi/malformed INI). Tag
 * add/remove round-trip and multi-INI listing/selection.
 */
describe('Fase 4b — Editor Edges', () => {
  let game: MockGame;
  let gameId: string;
  let modDir: string;

  before(async () => {
    game = await createMockGame('Phase4b');
    gameId = await seedGameAndOpenDashboard(game);
    await createObject(gameId, 'EditObj');
    modDir = await addMockMod(game, 'EditObj', 'EditMod');
    // Second INI file so multi-file listing has something to find.
    await fs.writeFile(path.join(modDir, 'extra.ini'), '[Constants]\n');
    await reconcile(gameId);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-17-02: Tags add then remove round-trips via metadata update', async () => {
    await invokeInApp('update_mod_info', {
      gameId,
      folderPath: modDir,
      update: { tags_add: ['favorite', 'wip'] },
    });
    let info = await invokeInApp<ModInfo>('read_mod_info', { gameId, folderPath: modDir });
    expect(info.tags).toContain('favorite');
    expect(info.tags).toContain('wip');

    await invokeInApp('update_mod_info', {
      gameId,
      folderPath: modDir,
      update: { tags_remove: ['wip'] },
    });
    info = await invokeInApp<ModInfo>('read_mod_info', { gameId, folderPath: modDir });
    expect(info.tags).toContain('favorite');
    expect(info.tags).not.toContain('wip');
  });

  it('TC-18-02: Multiple INI files are listed and each is readable', async () => {
    const files = await invokeInApp<{ filename: string }[]>('list_mod_ini_files', {
      gameId,
      folderPath: modDir,
    });
    const names = files.map((f) => f.filename);
    expect(names).toContain('mod.ini');
    expect(names).toContain('extra.ini');

    for (const fileName of names) {
      const doc = await invokeInApp('read_mod_ini', { gameId, folderPath: modDir, fileName });
      expect(doc).toBeDefined();
    }
  });
});
