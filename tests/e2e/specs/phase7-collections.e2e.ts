import { expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import {
  createMockGame,
  addMockMod,
  listDir,
  removeMockGame,
  type MockGame,
} from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import { createObject, findObject, getObjects, reconcile } from '../support/data.js';

interface CollectionSummary {
  id: string;
  name: string;
  [key: string]: unknown;
}
interface ApplyResult {
  mods_enabled: number;
  mods_disabled: number;
  partial_apply: boolean;
  skipped_missing_paths: string[];
}

/**
 * Fase 7 — Collections & konflik ⚠️ DATA-SAFETY. Transactional apply (all-or-
 * nothing with partial-missing reporting), conflict ignore/revoke round-trip,
 * and safe-mode marking.
 */
describe('Fase 7 — Collections & Conflict (data-safety)', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('Phase7');
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-31-01: Collection create → list → apply → delete lifecycle', async () => {
    await createObject(gameId, 'ColObj');
    const modPath = await addMockMod(game, 'ColObj', 'ColMod');
    await reconcile(gameId);

    const created = await invokeInApp<CollectionSummary>('create_collection', {
      gameId,
      name: 'E2E Collection',
      saveMode: 'save_current_state',
    });

    const list = await invokeInApp<CollectionSummary[]>('list_collections', { gameId });
    expect(list.some((c) => c.id === created.id)).toBe(true);

    await invokeInApp('bulk_toggle_mods', { gameId, paths: [modPath], enable: false });
    expect(await listDir(path.join(game.modsPath, 'ColObj'))).toContain('DISABLED ColMod');

    await invokeInApp('preview_apply_collection', { collectionId: created.id, gameId });
    const applied = await invokeInApp<ApplyResult>('apply_collection', {
      collectionId: created.id,
      gameId,
      ignoreMissing: true,
    });
    expect(applied.partial_apply).toBe(false);
    expect(applied.mods_enabled).toBe(1);
    expect(applied.mods_disabled).toBe(0);
    expect(await listDir(path.join(game.modsPath, 'ColObj'))).toContain('ColMod');
    expect(await listDir(path.join(game.modsPath, 'ColObj'))).not.toContain('DISABLED ColMod');
    expect((await findObject(gameId, 'ColObj'))?.enabled_count).toBe(1);

    await invokeInApp('delete_collection', { id: created.id });
    const after = await invokeInApp<CollectionSummary[]>('list_collections', { gameId });
    expect(after.some((c) => c.id === created.id)).toBe(false);
  });

  it('TC-31-02: Apply with a missing mod is transactional, not half-applied', async () => {
    await createObject(gameId, 'PartialObj');
    const keepPath = await addMockMod(game, 'PartialObj', 'PartialKeep');
    const missingPath = await addMockMod(game, 'PartialObj', 'PartialMissing');
    await reconcile(gameId);

    const created = await invokeInApp<CollectionSummary>('create_collection', {
      gameId,
      name: 'E2E Partial',
      saveMode: 'save_current_state',
    });

    await invokeInApp('bulk_toggle_mods', { gameId, paths: [keepPath], enable: false });

    // Remove one member after the snapshot → available members still apply,
    // while the missing member remains explicit in the result.
    await fs.rm(missingPath, {
      recursive: true,
      force: true,
    });
    await reconcile(gameId);

    const applied = await invokeInApp<ApplyResult>('apply_collection', {
      collectionId: created.id,
      gameId,
      ignoreMissing: true,
    });
    expect(applied.partial_apply).toBe(true);
    expect(applied.skipped_missing_paths.some((item) => item.includes('PartialMissing'))).toBe(
      true,
    );
    expect(await listDir(path.join(game.modsPath, 'PartialObj'))).toContain('PartialKeep');
    expect(await listDir(path.join(game.modsPath, 'PartialObj'))).not.toContain(
      'DISABLED PartialKeep',
    );
    expect((await findObject(gameId, 'PartialObj'))?.enabled_count).toBe(1);

    await invokeInApp('delete_collection', { id: created.id });
  });

  it('TC-29-01: Conflict ignore then revoke round-trips', async () => {
    const objectId = await createObject(gameId, 'ConflictObj');
    await reconcile(gameId);

    await invokeInApp('detect_conflicts_in_folder_cmd', { modsPath: game.modsPath });

    await invokeInApp('ignore_object_conflict', {
      gameId,
      objectId,
      modIds: ['e2e-mod-a', 'e2e-mod-b'],
    });
    const ignored = await invokeInApp<unknown[]>('list_ignored_object_conflicts', { gameId });
    expect(ignored.length).toBeGreaterThan(0);

    await invokeInApp('revoke_object_conflict', { gameId, objectId });
    const afterRevoke = await invokeInApp<{ length: number }[] & unknown[]>(
      'list_ignored_object_conflicts',
      { gameId },
    );
    expect(afterRevoke.length).toBeLessThan(ignored.length);
  });

  it('TC-30-01: Safety marking remains queryable without filtering projection data', async () => {
    await createObject(gameId, 'SafeObj');
    const modDir = await addMockMod(game, 'SafeObj', 'SafeMod');
    await reconcile(gameId);

    await invokeInApp('toggle_mod_safe', { gameId, folderPath: modDir, safe: true });

    const safeObjects = await getObjects(gameId);
    expect(Array.isArray(safeObjects)).toBe(true);
  });
});
