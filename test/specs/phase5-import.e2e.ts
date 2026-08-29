import { expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import { createMockGame, listDir, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { getObjects, reconcile } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';

interface ImportItem {
  id: string;
  plannedName: string;
  status: string;
}

interface ImportBatch {
  id: string;
  items: ImportItem[];
}

interface ImportBatchReport {
  moved: number;
  collisions: number;
  failed: number;
}

const FIXTURE_ZIP = path.resolve('test/fixtures/sample-mod.zip');

async function makeLooseFolder(root: string, name: string): Promise<string> {
  const dir = path.join(root, 'incoming', name);
  await fs.mkdir(dir, { recursive: true });
  await fs.writeFile(
    path.join(dir, 'mod.ini'),
    '[TextureOverrideMockMod]\nhash = 0123456789abcdef\n',
  );
  return dir;
}

describe('Fase 5 — Shared import batches (data-safety)', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('Phase5');
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  async function createTarget(name: string): Promise<{ id: string; path: string }> {
    const targetPath = path.join(game.modsPath, name);
    await fs.mkdir(targetPath, { recursive: true });
    await reconcile(gameId, 'ManualRepair');
    const object = (await getObjects(gameId)).find((candidate) => candidate.name === name);
    expect(object).toBeDefined();
    return { id: object!.id, path: targetPath };
  }

  async function prepareSpecificBatch(
    targetObjectId: string,
    sources: Array<{ path: string; sourceKind: 'folder' | 'archive_root' }>,
  ): Promise<ImportBatch> {
    const created = await invokeInApp<ImportBatch>('create_import_batch', {
      input: {
        gameId,
        flow: 'specific_import',
        targetMode: 'specific',
        targetObjectId,
        targetSubpath: null,
        sources,
      },
    });
    const analyzed = await invokeInApp<ImportBatch>('analyze_import_batch', {
      batchId: created.id,
    });

    for (const item of analyzed.items) {
      expect(item.status).toBe('awaiting_category');
      await invokeInApp('set_import_item_classification', {
        input: {
          itemId: item.id,
          category: 'Other',
          subCategory: null,
          metadata: {},
        },
      });
      await invokeInApp('refresh_import_item_suggestions', { itemId: item.id });
      await invokeInApp('set_import_item_decision', {
        input: {
          itemId: item.id,
          decision: 'keep_specific_target',
          destinationObjectId: targetObjectId,
          destinationPath: null,
          canonicalEntryKey: null,
          matchedAlias: null,
        },
      });
    }

    return invokeInApp<ImportBatch>('get_import_batch', { batchId: created.id });
  }

  it('TC-23-01: bulk specific import moves every confirmed folder through one batch', async () => {
    const target = await createTarget('ImportTarget');
    const ayaka = await makeLooseFolder(game.root, 'DISABLED ayaka-12319mods');
    const raiden = await makeLooseFolder(game.root, 'DISABLED raiden32114');
    const batch = await prepareSpecificBatch(target.id, [
      { path: ayaka, sourceKind: 'folder' },
      { path: raiden, sourceKind: 'folder' },
    ]);

    const report = await invokeInApp<ImportBatchReport>('commit_import_batch', {
      input: { batchId: batch.id, itemIds: batch.items.map((item) => item.id) },
    });

    expect(report.moved).toBe(2);
    expect(report.failed).toBe(0);
    expect(await listDir(target.path)).toEqual(
      expect.arrayContaining(['DISABLED ayaka-12319mods', 'DISABLED raiden32114']),
    );
    expect(await listDir(path.dirname(ayaka))).not.toContain('DISABLED ayaka-12319mods');
  });

  it('TC-39-01: collision skips the item and leaves the source untouched', async () => {
    const target = await createTarget('CollisionTarget');
    await fs.mkdir(path.join(target.path, 'DISABLED DupName'), { recursive: true });
    await fs.writeFile(path.join(target.path, 'DISABLED DupName', 'mod.ini'), '[Constants]\n');
    await reconcile(gameId, 'ManualRepair');
    const source = await makeLooseFolder(game.root, 'DupName');
    const batch = await prepareSpecificBatch(target.id, [{ path: source, sourceKind: 'folder' }]);

    const report = await invokeInApp<ImportBatchReport>('commit_import_batch', {
      input: { batchId: batch.id, itemIds: batch.items.map((item) => item.id) },
    });

    expect(report.collisions).toBe(1);
    expect(await listDir(source)).toContain('mod.ini');
  });

  it('TC-37-01: archive extraction stages roots and commits them through the same wizard contract', async () => {
    const target = await createTarget('ArchiveTarget');
    const archive = path.join(game.root, 'incoming', 'sample-mod.zip');
    await fs.mkdir(path.dirname(archive), { recursive: true });
    await fs.copyFile(FIXTURE_ZIP, archive);
    const batch = await prepareSpecificBatch(target.id, [
      { path: archive, sourceKind: 'archive_root' },
    ]);

    expect(batch.items.map((item) => item.plannedName)).toContain('DISABLED ArchivedMod');
    const report = await invokeInApp<ImportBatchReport>('commit_import_batch', {
      input: { batchId: batch.id, itemIds: batch.items.map((item) => item.id) },
    });

    expect(report.moved).toBe(1);
    expect(await listDir(target.path)).toContain('DISABLED ArchivedMod');
    expect(await fs.stat(archive)).toBeDefined();
  });
});
