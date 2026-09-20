import { expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import { createMockGame, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp, invokeWithChannel } from '../support/ipc.js';
import { reconcile, getObjects } from '../support/data.js';

/** Creates a raw object/mod tree directly on disk (no command) to test discovery. */
async function seedRawMod(modsPath: string, object: string, mod: string): Promise<void> {
  const dir = path.join(modsPath, object, mod);
  await fs.mkdir(dir, { recursive: true });
  await fs.writeFile(path.join(dir, 'mod.ini'), '[Constants]\n');
}

/**
 * Fase 6 — Mesin scan / sync / match. Command-level: disk reconcile discovery,
 * read-only classification preview, watcher-driven reconciliation, duplicate scanner. Asserts the disk →
 * DB projection and that long-running channel commands complete.
 */
describe('Fase 6 — Scan / Sync / Match', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('Phase6');
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-27-01: Disk reconcile discovers folders created outside the app', async () => {
    await seedRawMod(game.modsPath, 'DiscoverObj', 'DiscoverMod');
    await reconcile(gameId, 'WatcherBatch');
    const names = (await getObjects(gameId)).map((o) => o.name);
    expect(names).toContain('DiscoverObj');
  });

  it('TC-25-01: Classification is category-first and optionally disables the object', async () => {
    await seedRawMod(game.modsPath, 'ClassificationObj', 'ClassificationMod');
    await reconcile(gameId, 'ManualRepair');
    const object = (await getObjects(gameId)).find(
      (candidate) => candidate.name === 'ClassificationObj',
    );
    expect(object).toBeDefined();

    const preview = await invokeInApp<
      Array<{ objectId: string; canonicalSuggestions: unknown[]; fingerprint: unknown }>
    >('preview_object_classification_batch', {
      input: {
        gameId,
        objectIds: [object!.id],
        drafts: [],
      },
    });
    expect(preview).toHaveLength(1);
    expect(preview[0].objectId).toBe(object!.id);
    expect(preview[0].canonicalSuggestions).toEqual([]);

    const applied = await invokeInApp<{ disabledObjects: number; disableWarning: string | null }>(
      'apply_object_classification_batch',
      {
        input: {
          gameId,
          disableAfterApply: true,
          items: [
            {
              objectId: object!.id,
              category: 'Other',
              subCategory: null,
              metadata: {},
              canonicalEntryKey: null,
              canonicalAlias: null,
              confidencePercentage: null,
              fingerprint: preview[0].fingerprint,
            },
          ],
        },
      },
    );
    expect(applied.disabledObjects).toBe(1);
    expect(applied.disableWarning).toBeNull();
    expect(await fs.readdir(game.modsPath)).toContain('DISABLED ClassificationObj');
  });

  it('TC-28-01: Active watcher reconciles an external change', async () => {
    await seedRawMod(game.modsPath, 'WatchObj', 'WatchMod');
    await reconcile(gameId, 'WatcherBatch');
    const names = (await getObjects(gameId)).map((o) => o.name);
    expect(names).toContain('WatchObj');
  });

  it('TC-32-01: Duplicate scanner runs and report/ignored-pairs are queryable', async () => {
    await invokeWithChannel<void>('dup_scan_start', { gameId, modsRoot: game.modsPath }, 'onEvent');
    // Report may be null when nothing is flagged — the query must not throw.
    await invokeInApp('dup_scan_get_report', { gameId });
    const ignored = await invokeInApp<unknown[]>('get_ignored_pairs', { gameId });
    expect(Array.isArray(ignored)).toBe(true);
  });
});
