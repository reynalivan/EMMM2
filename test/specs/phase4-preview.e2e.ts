import { expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import { createMockGame, addMockMod, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import { createObject, reconcile } from '../support/data.js';

/** 1x1 transparent PNG. */
const PNG_BASE64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==';

interface ModInfo {
  author?: string | null;
  description?: string | null;
  [key: string]: unknown;
}

/**
 * Fase 4 — Preview & editor. Metadata (info.json), INI editor, image gallery,
 * thumbnail cache, keyviewer. Editors that write to disk are asserted by
 * re-reading the file from the filesystem.
 */
describe('Fase 4 — Preview & Editors', () => {
  let game: MockGame;
  let gameId: string;
  let modDir: string;
  let pngPath: string;

  before(async () => {
    game = await createMockGame('Phase4');
    gameId = await seedGameAndOpenDashboard(game);
    await createObject(gameId, 'PrevObj');
    modDir = await addMockMod(game, 'PrevObj', 'PrevMod');
    await reconcile(gameId);

    pngPath = path.join(game.root, 'sample.png');
    await fs.writeFile(pngPath, Buffer.from(PNG_BASE64, 'base64'));
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-17-01: Metadata editor round-trips author/description', async () => {
    await invokeInApp('update_mod_info', {
      folderPath: modDir,
      update: { author: 'E2E Author', description: 'phase 4 desc' },
    });
    const info = await invokeInApp<ModInfo>('read_mod_info', { folderPath: modDir });
    expect(info.author).toBe('E2E Author');
    expect(info.description).toBe('phase 4 desc');
  });

  it('TC-18-01: INI editor writes content that persists on disk', async () => {
    const files = await invokeInApp<string[]>('list_mod_ini_files', { folderPath: modDir });
    expect(files).toContain('mod.ini');

    await invokeInApp('write_mod_ini', {
      folderPath: modDir,
      fileName: 'mod.ini',
      content: '[Constants]\nglobal $active = 1\n',
    });

    const onDisk = await fs.readFile(path.join(modDir, 'mod.ini'), 'utf8');
    expect(onDisk).toContain('global $active = 1');
  });

  it('TC-19-01: Image gallery lists a saved preview image', async () => {
    const before = await invokeInApp<string[]>('list_mod_preview_images', { folderPath: modDir });
    await invokeInApp('save_mod_preview_image', { folderPath: modDir, imagePath: pngPath });
    const after = await invokeInApp<string[]>('list_mod_preview_images', { folderPath: modDir });
    expect(after.length).toBeGreaterThan(before.length);
  });

  it('TC-41-01: Thumbnail cache goes from empty to populated', async () => {
    const initial = await invokeInApp<string | null>('get_mod_thumbnail', {
      gameId,
      folderPath: modDir,
    });
    expect(initial).toBeNull();

    await invokeInApp('update_mod_thumbnail', { folderPath: modDir, sourcePath: pngPath });
    const populated = await invokeInApp<string | null>('get_mod_thumbnail', {
      gameId,
      folderPath: modDir,
    });
    expect(populated).not.toBeNull();
  });

  it('TC-43-01: Active keybindings command returns a list', async () => {
    const bindings = await invokeInApp<unknown[]>('get_active_keybindings', { gameId });
    expect(Array.isArray(bindings)).toBe(true);
  });
});
