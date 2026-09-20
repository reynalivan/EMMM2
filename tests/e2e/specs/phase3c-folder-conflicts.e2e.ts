import { browser, expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import {
  addMockMod,
  createMockGame,
  listDir,
  removeMockGame,
  type MockGame,
} from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { createObject, findObject } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';

interface DiskReconcileResult {
  status:
    'Applied' | 'AppliedWithFolderConflicts' | 'SourceUnavailable' | 'NeedsRenameConfirmation';
  folder_conflicts: {
    group_id: string;
    candidates: { path: string; base_name: string }[];
  }[];
  rename_confirmations: unknown[];
  path_updates: { from: string; to: string; kind: 'Object' | 'Mod' }[];
  change_summary: { mod_changes: { removed: number; renamed: number } };
}

interface FolderConflictMutationResult {
  reconcile: DiskReconcileResult | null;
  sync_warning: unknown | null;
}

async function rejects(command: string, args: Record<string, unknown>): Promise<void> {
  let threw = false;
  try {
    await invokeInApp(command, args);
  } catch {
    threw = true;
  }
  expect(threw).toBe(true);
}

describe('Fase 3c — Folder identity conflict recovery', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('FolderConflicts');
    gameId = await seedGameAndOpenDashboard(game);
    await createObject(gameId, 'ConflictA');
    await createObject(gameId, 'ConflictB');
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('blocks without SQL 1555, then resolves the queue through rename and Trash', async () => {
    await addMockMod(game, 'ConflictA', 'Mod A');
    await addMockMod(game, 'ConflictB', 'Mod B');
    const baseline = await invokeInApp<DiskReconcileResult>('reconcile_disk_state_cmd', {
      gameId,
      reason: 'ManualRepair',
      changedPaths: null,
      forceFull: true,
    });
    expect(baseline.status).toBe('Applied');
    expect((await findObject(gameId, 'ConflictB'))?.mod_count).toBe(1);

    await addMockMod(game, 'ConflictA', 'DISABLED Mod A');
    await addMockMod(game, 'ConflictB', 'DISABLED Mod B');

    const blockedResult = await invokeInApp<DiskReconcileResult>('reconcile_disk_state_cmd', {
      gameId,
      reason: 'ManualRepair',
      changedPaths: null,
      forceFull: true,
    });
    expect(blockedResult.status).toBe('AppliedWithFolderConflicts');
    expect(blockedResult.folder_conflicts).toHaveLength(2);

    expect(await browser.getPageSource()).not.toContain('UNIQUE constraint failed');

    const conflictAGroup = blockedResult.folder_conflicts.find((group) =>
      group.candidates.some((candidate) => candidate.path.includes('ConflictA')),
    );
    expect(conflictAGroup).toBeDefined();
    await rejects('open_in_explorer', {
      gameId,
      path: conflictAGroup!.candidates[0].path,
    });
    const details = await invokeInApp<unknown[]>('get_folder_conflict_details', {
      gameId,
      paths: conflictAGroup!.candidates.map((candidate) => candidate.path),
    });
    expect(details).toHaveLength(2);

    const afterRename = await invokeInApp<DiskReconcileResult>('resolve_folder_name_conflict', {
      gameId,
      groupId: conflictAGroup!.group_id,
      renames: conflictAGroup!.candidates.map((candidate) => ({
        path: candidate.path,
        base_name: candidate.path.includes('DISABLED ') ? 'Mod A Main' : 'Mod A Archive',
      })),
    });
    expect(afterRename.status).toBe('AppliedWithFolderConflicts');
    expect(afterRename.folder_conflicts).toHaveLength(1);
    await rejects('trash_folder_conflict_candidate', {
      gameId,
      path: conflictAGroup!.candidates[0].path,
    });

    const trashedPath = afterRename.folder_conflicts[0].candidates[0].path;
    const relativeTrashedPath = path.relative(game.modsPath, trashedPath);
    const trashResult = await invokeInApp<FolderConflictMutationResult>(
      'trash_folder_conflict_candidate',
      {
        gameId,
        path: relativeTrashedPath,
      },
    );
    expect(trashResult.sync_warning).toBeNull();
    const afterTrash = trashResult.reconcile;
    expect(afterTrash).not.toBeNull();
    if (!afterTrash) throw new Error('Trash committed without a reconcile result');
    expect(afterTrash.status).toBe('Applied');
    expect(afterTrash.folder_conflicts).toHaveLength(0);
    expect((await findObject(gameId, 'ConflictB'))?.mod_count).toBe(1);

    const finalResult = await invokeInApp<DiskReconcileResult>('reconcile_disk_state_cmd', {
      gameId,
      reason: 'ManualRepair',
      changedPaths: null,
      forceFull: true,
    });
    expect(finalResult.status).toBe('Applied');
    expect(finalResult.folder_conflicts).toHaveLength(0);
    expect(await browser.getPageSource()).not.toContain('UNIQUE constraint failed');

    const conflictA = path.join(game.modsPath, 'ConflictA');
    expect(await listDir(conflictA)).toEqual(
      expect.arrayContaining(['DISABLED Mod A Main', 'Mod A Archive']),
    );
    expect((await listDir(path.join(game.modsPath, 'ConflictB'))).length).toBe(1);
  });

  it('rejects an in-app rename that would create a normalized identity conflict', async () => {
    await createObject(gameId, 'RenameGuard');
    const sourcePath = await addMockMod(game, 'RenameGuard', 'Blue');
    await addMockMod(game, 'RenameGuard', 'DISABLED Red');
    const baseline = await invokeInApp<DiskReconcileResult>('reconcile_disk_state_cmd', {
      gameId,
      reason: 'ManualRepair',
      changedPaths: null,
      forceFull: true,
    });
    expect(baseline.status).toBe('Applied');

    await rejects('rename_mod_folder', {
      folderPath: sourcePath,
      newName: 'Red',
      gameId,
    });

    expect(await listDir(path.join(game.modsPath, 'RenameGuard'))).toEqual(
      expect.arrayContaining(['Blue', 'DISABLED Red']),
    );
    const afterRejectedRename = await invokeInApp<DiskReconcileResult>('reconcile_disk_state_cmd', {
      gameId,
      reason: 'ManualRepair',
      changedPaths: null,
      forceFull: true,
    });
    expect(afterRejectedRename.status).toBe('Applied');
    expect(afterRejectedRename.folder_conflicts).toHaveLength(0);
  });

  it('heals a deep nested rename from the active watcher projection', async () => {
    await createObject(gameId, 'OfflineRename');
    const oldPath = await addMockMod(game, path.join('OfflineRename', 'Variants'), 'Old Style');
    const baseline = await invokeInApp<DiskReconcileResult>('reconcile_disk_state_cmd', {
      gameId,
      reason: 'ManualRepair',
      changedPaths: null,
      forceFull: true,
    });
    expect(baseline.status).toBe('Applied');

    const newPath = path.join(game.modsPath, 'OfflineRename', 'Variants', 'New Style');
    await fs.rename(oldPath, newPath);
    const recovered = await invokeInApp<DiskReconcileResult>('reconcile_disk_state_cmd', {
      gameId,
      reason: 'StartupBoot',
      changedPaths: null,
      forceFull: true,
    });

    expect(recovered.status).toBe('Applied');
    expect(recovered.rename_confirmations).toHaveLength(0);
    expect(recovered.change_summary.mod_changes.removed).toBe(0);
    expect(recovered.change_summary.mod_changes.renamed).toBe(1);
    expect(
      recovered.path_updates.some(
        (update) =>
          update.kind === 'Mod' &&
          update.from.replaceAll('\\', '/').endsWith('OfflineRename/Variants/Old Style') &&
          update.to.replaceAll('\\', '/').endsWith('OfflineRename/Variants/New Style'),
      ),
    ).toBe(true);
    expect((await findObject(gameId, 'OfflineRename'))?.mod_count).toBe(1);
  });
});
