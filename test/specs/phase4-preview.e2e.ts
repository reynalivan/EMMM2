import { expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import { createMockGame, addMockMod, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import { createObject, reconcile } from '../support/data.js';

/**
 * 1x1 RGBA PNG. Must stay a structurally complete PNG (IHDR/IDAT/IEND) — the
 * backend really decodes it, and a truncated blob fails with
 * "image decode/encode failed: Format error decoding PNG".
 */
const PNG_BASE64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==';

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
      gameId,
      folderPath: modDir,
      update: { author: 'E2E Author', description: 'phase 4 desc' },
    });
    const info = await invokeInApp<ModInfo>('read_mod_info', { gameId, folderPath: modDir });
    expect(info.author).toBe('E2E Author');
    expect(info.description).toBe('phase 4 desc');
  });

  it('TC-18-01: INI editor writes content that persists on disk', async () => {
    const files = await invokeInApp<{ filename: string }[]>('list_mod_ini_files', {
      gameId,
      folderPath: modDir,
    });
    expect(files.map((f) => f.filename)).toContain('mod.ini');

    // The editor patches individual lines, it does not overwrite the file with
    // a blob. Line 1 of the seeded `mod.ini` is the `hash = ...` line.
    await invokeInApp('write_mod_ini', {
      gameId,
      folderPath: modDir,
      fileName: 'mod.ini',
      lineUpdates: [{ line_idx: 1, content: 'global $active = 1' }],
    });

    const onDisk = await fs.readFile(path.join(modDir, 'mod.ini'), 'utf8');
    expect(onDisk).toContain('global $active = 1');
  });

  it('TC-19-01: Image gallery lists a saved preview image', async () => {
    const before = await invokeInApp<string[]>('list_mod_preview_images', {
      gameId,
      folderPath: modDir,
    });
    await invokeInApp('save_mod_preview_image', {
      gameId,
      folderPath: modDir,
      objectName: 'PrevObj',
      imageData: Array.from(Buffer.from(PNG_BASE64, 'base64')),
    });
    const after = await invokeInApp<string[]>('list_mod_preview_images', {
      gameId,
      folderPath: modDir,
    });
    expect(after.length).toBeGreaterThan(before.length);
  });

  it('TC-41-01: Thumbnail cache goes from empty to populated', async () => {
    // Its own mod folder: TC-19-01 saves a preview image into `modDir`, and a
    // preview image is enough for `get_mod_thumbnail` to return a path — so
    // reusing it would start this test already populated.
    const freshDir = await addMockMod(game, 'PrevObj', 'ThumbMod');
    await reconcile(gameId);

    const initial = await invokeInApp<string | null>('get_mod_thumbnail', {
      gameId,
      folderPath: freshDir,
    });
    expect(initial).toBeNull();

    await invokeInApp('update_mod_thumbnail', {
      gameId,
      folderPath: freshDir,
      sourcePath: pngPath,
    });
    const populated = await invokeInApp<string | null>('get_mod_thumbnail', {
      gameId,
      folderPath: freshDir,
    });
    expect(populated).not.toBeNull();
  });

  it('TC-43-01: Active keybindings command returns a list', async () => {
    const bindings = await invokeInApp<unknown[]>('get_active_keybindings', { gameId });
    expect(Array.isArray(bindings)).toBe(true);
  });
});
