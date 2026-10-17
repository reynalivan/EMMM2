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
import { reconcile } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';

/** Minimal shapes for the command return values this smoke asserts on. */
interface BulkResult {
  success: string[];
  failures: { path: string; error: unknown }[];
}

/**
 * TC-00 — Production smoke for the data-safety paths (see
 * `docs/test-cases/tc-00-production-smoke.md` and `e2e-scope.md` Phase 0).
 *
 * Runs at the IPC layer with two-sided asserts: verify the DISK (rename prefix,
 * trash location, file integrity) AND the command return / read-back. Deep
 * per-mod DB status assertions live in Phase 3 (tc-20 / tc-22).
 */
describe('TC-00 Smoke — Data-Safety Paths', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('Smoke');
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  beforeEach(async () => {
    await reconcile(gameId);
  });

  it('TC-00-01: Toggle off/on renames DISABLED prefix on disk, no file lost', async () => {
    const objectDir = path.join(game.modsPath, 'Raiden');
    const modPath = await addMockMod(game, 'Raiden', 'SkinA');

    const off = await invokeInApp<BulkResult>('bulk_toggle_mods', {
      gameId,
      paths: [modPath],
      enable: false,
      operationId: 'smoke-data-safety-disable',
      intentRevision: 1,
      expectedIdentities: null,
    });
    expect(off.failures.length).toBe(0);

    let entries = await listDir(objectDir);
    expect(entries).toContain('DISABLED SkinA');
    expect(entries).not.toContain('SkinA');
    // File integrity: contents survive the rename.
    expect(await listDir(path.join(objectDir, 'DISABLED SkinA'))).toContain('mod.ini');

    // The first committed mutation stages watcher work. Settle it before
    // submitting the inverse mutation so the test mirrors UI action sequencing.
    await reconcile(gameId);

    const on = await invokeInApp<BulkResult>('bulk_toggle_mods', {
      gameId,
      paths: [path.join(objectDir, 'DISABLED SkinA')],
      enable: true,
      operationId: 'smoke-data-safety-enable',
      intentRevision: 2,
      expectedIdentities: null,
    });
    expect(on.failures.length).toBe(0);

    entries = await listDir(objectDir);
    expect(entries).toContain('SkinA');
    expect(entries).not.toContain('DISABLED SkinA');
  });

  it('TC-00-02: Delete removes the mod through native soft-delete', async () => {
    const objectDir = path.join(game.modsPath, 'Nahida');
    await addMockMod(game, 'Nahida', 'SkinB');
    const modPath = path.join(objectDir, 'SkinB');

    await invokeInApp('delete_mod', { path: modPath, gameId });

    // Disk: removed from the object folder...
    expect(await listDir(objectDir)).not.toContain('SkinB');

    // Recovery is owned by the Windows Recycle Bin and covered by manual smoke.
  });

  it('TC-00-03: Disk reconcile after mutations completes without error', async () => {
    const result = await invokeInApp('reconcile_disk_state_cmd', {
      gameId,
      reason: 'ManualRepair',
      forceFull: true,
    });
    expect(result).toBeDefined();
  });
});
