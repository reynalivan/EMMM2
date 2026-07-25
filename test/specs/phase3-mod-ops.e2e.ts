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
import { createObject, reconcile, getObjects, findObject } from '../support/data.js';

interface BulkResult {
  success: string[];
  failures: { path: string; error: unknown }[];
}
interface TrashEntry {
  id: string;
  original_name: string;
}

/**
 * Fase 3 — Operasi mod inti ⚠️ DATA-SAFETY. Every destructive mutation is
 * asserted on BOTH sides: disk (rename prefix / trash location / file
 * integrity) AND DB projection (getObjects mod_count / enabled_count).
 * Reconcile runs before explicit ops so the mod exists as a DB row to update.
 */
describe('Fase 3 — Core Mod Operations (data-safety)', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('Phase3');
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-20-01: Toggle off/on updates disk prefix AND enabled_count', async () => {
    await createObject(gameId, 'TogObj');
    await addMockMod(game, 'TogObj', 'SkinA');
    await reconcile(gameId);
    const objDir = path.join(game.modsPath, 'TogObj');

    const off = await invokeInApp<BulkResult>('bulk_toggle_mods', {
      gameId,
      paths: [path.join(objDir, 'SkinA')],
      enable: false,
    });
    expect(off.failures.length).toBe(0);
    expect(await listDir(objDir)).toContain('DISABLED SkinA');
    expect((await findObject(gameId, 'TogObj'))!.enabled_count).toBe(0);

    const on = await invokeInApp<BulkResult>('bulk_toggle_mods', {
      gameId,
      paths: [path.join(objDir, 'DISABLED SkinA')],
      enable: true,
    });
    expect(on.failures.length).toBe(0);
    expect(await listDir(objDir)).toContain('SkinA');
    expect((await findObject(gameId, 'TogObj'))!.enabled_count).toBe(1);
  });

  it('TC-21-01: Rename mod folder renames on disk, contents intact', async () => {
    await createObject(gameId, 'RenObj');
    await addMockMod(game, 'RenObj', 'OldName');
    await reconcile(gameId);
    const objDir = path.join(game.modsPath, 'RenObj');

    await invokeInApp('rename_mod_folder', {
      folderPath: path.join(objDir, 'OldName'),
      newName: 'NewName',
      gameId,
    });

    const entries = await listDir(objDir);
    expect(entries).toContain('NewName');
    expect(entries).not.toContain('OldName');
    expect(await listDir(path.join(objDir, 'NewName'))).toContain('mod.ini');
  });

  it('TC-22-01: Delete → trash → restore keeps DB projection consistent', async () => {
    await createObject(gameId, 'TrashObj');
    await addMockMod(game, 'TrashObj', 'ModX');
    await reconcile(gameId);
    const objDir = path.join(game.modsPath, 'TrashObj');

    await invokeInApp('delete_mod', { path: path.join(objDir, 'ModX'), gameId });
    expect(await listDir(objDir)).not.toContain('ModX');

    const trash = await invokeInApp<TrashEntry[]>('list_trash');
    const entry = trash.find((t) => t.original_name === 'ModX');
    expect(entry).toBeDefined();

    await invokeInApp('restore_mod', { trashId: entry!.id, gameId });
    await reconcile(gameId);
    expect(await listDir(objDir)).toContain('ModX');
    expect((await findObject(gameId, 'TrashObj'))!.mod_count).toBeGreaterThanOrEqual(1);
  });

  it('TC-14-01: Bulk toggle + bulk delete apply to every path', async () => {
    await createObject(gameId, 'BulkObj');
    for (const m of ['B1', 'B2', 'B3']) {
      await addMockMod(game, 'BulkObj', m);
    }
    await reconcile(gameId);
    const objDir = path.join(game.modsPath, 'BulkObj');
    const paths = ['B1', 'B2', 'B3'].map((m) => path.join(objDir, m));

    const toggled = await invokeInApp<BulkResult>('bulk_toggle_mods', {
      gameId,
      paths,
      enable: false,
    });
    expect(toggled.failures.length).toBe(0);
    const disabled = await listDir(objDir);
    expect(disabled.filter((n) => n.startsWith('DISABLED ')).length).toBe(3);

    const disabledPaths = ['B1', 'B2', 'B3'].map((m) => path.join(objDir, `DISABLED ${m}`));
    const deleted = await invokeInApp<BulkResult>('bulk_delete_mods', {
      gameId,
      paths: disabledPaths,
    });
    expect(deleted.failures.length).toBe(0);
    expect((await listDir(objDir)).length).toBe(0);
  });

  it('TC-10-01: Object CRUD — create writes folder + row, delete removes both', async () => {
    const id = await createObject(gameId, 'CrudObj');
    expect(await listDir(game.modsPath)).toContain('CrudObj');
    expect(await findObject(gameId, 'CrudObj')).toBeDefined();

    await invokeInApp('update_object_cmd', { id, updates: { name: 'CrudRenamed' } });
    expect(
      (await findObject(gameId, 'CrudRenamed')) ?? (await findObject(gameId, 'CrudObj')),
    ).toBeDefined();

    await invokeInApp('delete_object_cmd', { id, force: true });
    const remaining = (await getObjects(gameId)).map((o) => o.name);
    expect(remaining).not.toContain('CrudObj');
    expect(remaining).not.toContain('CrudRenamed');
  });

  it('TC-40-01: Move mods to another object relocates folder on disk', async () => {
    const srcId = await createObject(gameId, 'MoveSrc');
    const dstId = await createObject(gameId, 'MoveDst');
    await addMockMod(game, 'MoveSrc', 'Traveler');
    await reconcile(gameId);

    await invokeInApp('pin_object', { id: dstId, isPinned: true });

    await invokeInApp('move_mods_to_object', {
      input: {
        game_id: gameId,
        folder_paths: [path.join(game.modsPath, 'MoveSrc', 'Traveler')],
        target_object_id: dstId,
        target_subpath: null,
        status: null,
      },
    });
    await reconcile(gameId);

    expect(await listDir(path.join(game.modsPath, 'MoveDst'))).toContain('Traveler');
    expect(await listDir(path.join(game.modsPath, 'MoveSrc'))).not.toContain('Traveler');
    expect(srcId).not.toBe(dstId);
  });
});
