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

/** Minimal shapes for the command return values this smoke asserts on. */
interface BulkResult {
  success: string[];
  failures: { path: string; error: unknown }[];
}
interface TrashEntry {
  id: string;
  original_name: string;
}

/**
 * TC-00 — Production smoke for the data-safety paths (see
 * `.docs/.testcase/tc-00-production-smoke.md` and `e2e-scope.md` Fase 0).
 *
 * Runs at the IPC layer with two-sided asserts: verify the DISK (rename prefix,
 * trash location, file integrity) AND the command return / read-back. Deep
 * per-mod DB status assertions live in Fase 3 (tc-20 / tc-22).
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

  it('TC-00-01: Toggle off/on renames DISABLED prefix on disk, no file lost', async () => {
    const objectDir = path.join(game.modsPath, 'Raiden');
    const modPath = await addMockMod(game, 'Raiden', 'SkinA');

    const off = await invokeInApp<BulkResult>('bulk_toggle_mods', {
      gameId,
      paths: [modPath],
      enable: false,
    });
    expect(off.failures.length).toBe(0);

    let entries = await listDir(objectDir);
    expect(entries).toContain('DISABLED SkinA');
    expect(entries).not.toContain('SkinA');
    // File integrity: contents survive the rename.
    expect(await listDir(path.join(objectDir, 'DISABLED SkinA'))).toContain('mod.ini');

    const on = await invokeInApp<BulkResult>('bulk_toggle_mods', {
      gameId,
      paths: [path.join(objectDir, 'DISABLED SkinA')],
      enable: true,
    });
    expect(on.failures.length).toBe(0);

    entries = await listDir(objectDir);
    expect(entries).toContain('SkinA');
    expect(entries).not.toContain('DISABLED SkinA');
  });

  it('TC-00-02: Delete moves mod to trash (no hard delete) and restores intact', async () => {
    const objectDir = path.join(game.modsPath, 'Nahida');
    await addMockMod(game, 'Nahida', 'SkinB');
    const modPath = path.join(objectDir, 'SkinB');

    await invokeInApp('delete_mod', { path: modPath, gameId });

    // Disk: removed from the object folder...
    expect(await listDir(objectDir)).not.toContain('SkinB');

    // ...but present in trash (soft delete, recoverable).
    const trash = await invokeInApp<TrashEntry[]>('list_trash');
    const entry = trash.find((t) => t.original_name === 'SkinB');
    expect(entry).toBeDefined();

    // Restore returns the folder with its contents intact.
    await invokeInApp('restore_mod', { trashId: entry!.id, gameId });
    expect(await listDir(objectDir)).toContain('SkinB');
    expect(await listDir(modPath)).toContain('mod.ini');
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
