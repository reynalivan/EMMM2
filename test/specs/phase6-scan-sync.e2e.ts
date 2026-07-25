import { expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import { createMockGame, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { invokeInApp, invokeWithChannel } from '../support/ipc.js';
import { reconcile, getObjects } from '../support/data.js';

interface GameConfig {
  id: string;
  game_type: number;
  [key: string]: unknown;
}

/** Creates a raw object/mod tree directly on disk (no command) to test discovery. */
async function seedRawMod(modsPath: string, object: string, mod: string): Promise<void> {
  const dir = path.join(modsPath, object, mod);
  await fs.mkdir(dir, { recursive: true });
  await fs.writeFile(path.join(dir, 'mod.ini'), '[Constants]\n');
}

/**
 * Fase 6 — Mesin scan / sync / match. Command-level: disk reconcile discovery,
 * deep-match scanner, watcher lifecycle, duplicate scanner. Asserts the disk →
 * DB projection and that long-running channel commands complete.
 */
describe('Fase 6 — Scan / Sync / Match', () => {
  let game: MockGame;
  let gameId: string;
  let gameType: number;

  before(async () => {
    game = await createMockGame('Phase6');
    gameId = await seedGameAndOpenDashboard(game);
    const games = await invokeInApp<GameConfig[]>('get_games');
    gameType = games.find((g) => g.id === gameId)!.game_type;
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

  it('TC-25-01: Deep-match scanner completes and returns a sync result', async () => {
    const dbJson = await invokeInApp<string>('get_master_db', { gameType });
    const { value } = await invokeWithChannel<unknown>(
      'deepmatch_scanner_cmd',
      {
        gameId,
        gameName: 'E2E',
        gameType: 'GIMI',
        modsPath: game.modsPath,
        dbJson,
        preserveExistingMappings: true,
      },
      'onProgress',
    );
    expect(value).toBeDefined();
  });

  it('TC-28-01: Watcher start/stop lifecycle and external-change reconcile', async () => {
    await invokeInApp('start_watcher', { gameId, path: game.modsPath });
    try {
      await seedRawMod(game.modsPath, 'WatchObj', 'WatchMod');
      await reconcile(gameId, 'WatcherBatch');
      const names = (await getObjects(gameId)).map((o) => o.name);
      expect(names).toContain('WatchObj');
    } finally {
      await invokeInApp('stop_watcher');
    }
  });

  it('TC-32-01: Duplicate scanner runs and report/ignored-pairs are queryable', async () => {
    await invokeWithChannel<void>('dup_scan_start', { gameId, modsRoot: game.modsPath }, 'onEvent');
    // Report may be null when nothing is flagged — the query must not throw.
    await invokeInApp('dup_scan_get_report', {});
    const ignored = await invokeInApp<unknown[]>('get_ignored_pairs', { gameId });
    expect(Array.isArray(ignored)).toBe(true);
  });
});
