import { $, browser, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
import type {
  AppSettings,
  ImportBatch,
  ImportBatchReport,
} from '../../../src/shared/api/tauri/bindings.gen.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { createObject, waitForWorkspaceCoreReady } from '../support/data.js';
import { createMockGame, scheduleMockGameRemoval, type MockGame } from '../support/fixtures.js';
import { invokeInApp } from '../support/ipc.js';

describe('Ingestion package boundaries and Utility Skip', function () {
  this.timeout(90_000);
  let game: MockGame;
  let gameId: string;
  let inbox: string;

  before(async () => {
    game = await createMockGame(
      'IngestionBoundary',
      path.join(os.tmpdir(), `EMMM_Ingestion_${randomUUID()}`, 'GIMI'),
    );
    scheduleMockGameRemoval(game);
    inbox = path.join(game.fixtureRoot, 'Inbox');
    await fs.mkdir(inbox);
    gameId = await seedGameAndOpenDashboard(game);
    const settings = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', {
      settings: {
        ...settings,
        games: settings.games.map((configured) =>
          configured.id === gameId ? { ...configured, ready_to_move_path: inbox } : configured,
        ),
      },
    });
    await browser.setWindowSize(1360, 900);
  });

  async function analyze(source: string, targetId: string): Promise<ImportBatch> {
    const created = await invokeInApp<ImportBatch>('create_import_batch', {
      input: {
        gameId,
        flow: 'ready_to_move',
        targetMode: 'specific',
        targetObjectId: targetId,
        targetSubpath: null,
        sources: [{ path: source, sourceKind: 'ready_to_move' }],
      },
    });
    const analyzed = await invokeInApp<ImportBatch>('analyze_import_batch', {
      batchId: created.id,
    });
    for (const item of analyzed.items) {
      if (item.status === 'awaiting_category') {
        await invokeInApp('set_import_item_classification', {
          input: { itemId: item.id, category: 'Other', subCategory: null, metadata: {} },
        });
        await invokeInApp('refresh_import_item_suggestions', { itemId: item.id });
      }
    }
    return invokeInApp<ImportBatch>('get_import_batch', { batchId: created.id });
  }

  for (const multiple of [false, true]) {
    it(`commits ${multiple ? 'multiple roots' : 'one nested root'} with referenced Shared assets`, async () => {
      const name = multiple ? 'MultiBoundary' : 'SingleBoundary';
      const targetId = await createObject(gameId, `${name}Target`, 'Other');
      const source = path.join(inbox, name);
      const roots = multiple ? ['Variants/A', 'Variants/B'] : ['Body'];
      const reference = multiple ? '../../Shared/texture.dds' : '../Shared/texture.dds';
      await fs.mkdir(path.join(source, 'Shared'), { recursive: true });
      await fs.writeFile(path.join(source, 'Shared/texture.dds'), name);
      for (const root of roots) {
        await fs.mkdir(path.join(source, root), { recursive: true });
        await fs.writeFile(
          path.join(source, root, 'merged.ini'),
          `[TextureOverride${name}]\nhash = ${multiple ? 'aabb1122' : 'ccdd3344'}\n[ResourceBody]\nfilename = ${reference}\n`,
        );
      }
      const batch = await analyze(source, targetId);
      expect(batch.items.length).toBe(1);
      const item = batch.items[0];
      await invokeInApp('set_import_item_decision', {
        input: {
          itemId: item.id,
          decision: 'keep_specific_target',
          destinationObjectId: targetId,
          destinationPath: null,
          canonicalEntryKey: null,
          matchedAlias: null,
        },
      });
      await invokeInApp('mark_import_batch_review_started', { batchId: batch.id });
      const report = await invokeInApp<ImportBatchReport>('commit_import_batch', {
        input: { batchId: batch.id, itemIds: [item.id] },
      });
      expect(report.moved).toBe(1);
      expect(report.failed).toBe(0);
      await waitForWorkspaceCoreReady(gameId);
      const installed = path.join(game.modsPath, `${name}Target`, item.plannedName);
      expect(await fs.readFile(path.join(installed, 'Shared/texture.dds'), 'utf8')).toBe(name);
      for (const root of roots) {
        expect(await fs.readFile(path.join(installed, root, reference), 'utf8')).toBe(name);
      }
    });
  }

  it('skips Utility content through the existing wizard button', async () => {
    const targetId = await createObject(gameId, 'UtilityTarget', 'Other');
    const source = path.join(inbox, 'UtilityTimer');
    await fs.mkdir(source);
    await fs.writeFile(
      path.join(source, 'timer.ini'),
      '[CommandListTimer]\nrun = CommandListTick\n',
    );
    const batch = await analyze(source, targetId);
    expect(batch.items[0].contentKind).toBe('utility');
    await invokeInApp('mark_import_batch_review_started', { batchId: batch.id });
    await browser.url('http://tauri.localhost/');
    const resume = await $('button*=Resume import');
    await resume.waitForClickable({ timeout: 25_000 });
    await resume.click();
    const skip = await $('button=Skip');
    await skip.waitForClickable({ timeout: 25_000 });
    await skip.click();
    await browser.waitUntil(
      async () => {
        const updated = await invokeInApp<ImportBatch>('get_import_batch', { batchId: batch.id });
        return updated.items[0].status === 'skipped';
      },
      { timeout: 15_000, timeoutMsg: 'Utility Skip did not persist' },
    );
    const updated = await invokeInApp<ImportBatch>('get_import_batch', { batchId: batch.id });
    const skipped = updated.items[0];
    expect(skipped.decision).toBe('skip');
    expect(skipped.destinationPath).toBeNull();
    expect(await fs.readFile(path.join(source, 'timer.ini'), 'utf8')).toContain(
      '[CommandListTimer]',
    );
  });
});
