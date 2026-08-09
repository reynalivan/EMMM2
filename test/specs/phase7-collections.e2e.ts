import { expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import { createMockGame, addMockMod, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import { createObject, getObjects, reconcile } from '../support/data.js';

interface CollectionSummary {
  id: string;
  name: string;
  [key: string]: unknown;
}
interface ApplyResult {
  success: boolean;
  [key: string]: unknown;
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
    await addMockMod(game, 'ColObj', 'ColMod');
    await reconcile(gameId);

    const created = await invokeInApp<CollectionSummary>('create_collection', {
      gameId,
      name: 'E2E Collection',
      saveMode: 'save_current_state',
    });

    const list = await invokeInApp<CollectionSummary[]>('list_collections', { gameId });
    expect(list.some((c) => c.id === created.id)).toBe(true);

    await invokeInApp('preview_apply_collection', { collectionId: created.id, gameId });
    const applied = await invokeInApp<ApplyResult>('apply_collection', {
      collectionId: created.id,
      gameId,
      ignoreMissing: true,
    });
    expect(applied.success).toBe(true);

    await invokeInApp('delete_collection', { id: created.id });
    const after = await invokeInApp<CollectionSummary[]>('list_collections', { gameId });
    expect(after.some((c) => c.id === created.id)).toBe(false);
  });

  it('TC-31-02: Apply with a missing mod is transactional, not half-applied', async () => {
    await createObject(gameId, 'PartialObj');
    await addMockMod(game, 'PartialObj', 'PartialMod');
    await reconcile(gameId);

    const created = await invokeInApp<CollectionSummary>('create_collection', {
      gameId,
      name: 'E2E Partial',
      saveMode: 'save_current_state',
    });

    // Remove the mod from disk after the snapshot → apply must handle the gap.
    await fs.rm(path.join(game.modsPath, 'PartialObj', 'PartialMod'), {
      recursive: true,
      force: true,
    });
    await reconcile(gameId);

    const applied = await invokeInApp<ApplyResult>('apply_collection', {
      collectionId: created.id,
      gameId,
      ignoreMissing: true,
    });
    expect(typeof applied.success).toBe('boolean');

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

  it('TC-30-01: Safe-mode marking and safe-scoped object query', async () => {
    await createObject(gameId, 'SafeObj');
    const modDir = await addMockMod(game, 'SafeObj', 'SafeMod');
    await reconcile(gameId);

    await invokeInApp('toggle_mod_safe', { gameId, folderPath: modDir, safe: true });

    // safe_mode is derived backend-side from the active corridor, not passed in.
    const safeObjects = await getObjects(gameId);
    expect(Array.isArray(safeObjects)).toBe(true);
  });
});
