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

interface BulkResult {
  success: string[];
  failures: { path: string; error: unknown }[];
}
interface TrashEntry {
  id: string;
  original_name: string;
}

/** Asserts a command rejects (errors bubble up — no silent success on bad input). */
async function rejects(cmd: string, args: Record<string, unknown>): Promise<void> {
  let threw = false;
  try {
    await invokeInApp(cmd, args);
  } catch {
    threw = true;
  }
  expect(threw).toBe(true);
}

/**
 * Fase 3b — Failure & validation paths (tc-13 / tc-10 / tc-14 / tc-22 edges).
 * Complements the happy-path Fase 3: invalid input must be rejected, bad paths
 * must surface as per-item failures, and trash must empty cleanly. All IPC-level
 * and fully grounded in the backend's validation contracts.
 */
describe('Fase 3b — Failure & Validation Paths (data-safety)', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('Phase3b');
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-21-02: Rename with reserved characters is rejected', async () => {
    await createObject(gameId, 'RenFail');
    await addMockMod(game, 'RenFail', 'GoodName');
    await reconcile(gameId);
    const modPath = path.join(game.modsPath, 'RenFail', 'GoodName');

    await rejects('rename_mod_folder', { folderPath: modPath, newName: 'bad/name', gameId });
    await rejects('rename_mod_folder', { folderPath: modPath, newName: 'bad:name?', gameId });
    await rejects('rename_mod_folder', { folderPath: modPath, newName: '', gameId });

    // Original folder must be untouched after every rejected rename.
    expect(await listDir(path.join(game.modsPath, 'RenFail'))).toContain('GoodName');
  });

  it('TC-10-02: Creating a duplicate object name is rejected', async () => {
    await createObject(gameId, 'DupObj');
    await rejects('create_object_cmd', {
      input: { game_id: gameId, name: 'DupObj', object_type: 'Character' },
    });
  });

  it('TC-14-02: Bulk delete reports per-item failure, valid items still succeed', async () => {
    await createObject(gameId, 'PartialBulk');
    await addMockMod(game, 'PartialBulk', 'RealMod');
    await reconcile(gameId);
    const objDir = path.join(game.modsPath, 'PartialBulk');

    const res = await invokeInApp<BulkResult>('bulk_delete_mods', {
      gameId,
      paths: [path.join(objDir, 'RealMod'), path.join(objDir, 'DoesNotExist')],
    });

    expect(res.success.length).toBe(1);
    expect(res.failures.length).toBe(1);
    // The valid mod really left disk (moved to trash), not left half-done.
    expect(await listDir(objDir)).not.toContain('RealMod');
  });

  it('TC-22-02: Empty trash clears entries and returns a count', async () => {
    await createObject(gameId, 'EmptyTrashObj');
    await addMockMod(game, 'EmptyTrashObj', 'ToPurge');
    await reconcile(gameId);

    await invokeInApp('delete_mod', {
      path: path.join(game.modsPath, 'EmptyTrashObj', 'ToPurge'),
      gameId,
    });
    const before = await invokeInApp<TrashEntry[]>('list_trash');
    expect(before.length).toBeGreaterThan(0);

    const removed = await invokeInApp<number>('empty_trash');
    expect(removed).toBeGreaterThan(0);
    expect((await invokeInApp<TrashEntry[]>('list_trash')).length).toBe(0);
  });
});
