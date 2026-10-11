import { $, browser, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {
  addMockMod,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { getObjects } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';

const CASE_NAMES = ['last_owner_release', 'echo_expiry', 'echo_abort'] as const;

interface WatcherVerificationCase {
  name: (typeof CASE_NAMES)[number];
  session_generation: number;
  before_revision: number;
  accepted_revision: number;
  reconcile_event_revision: number;
  runtime_generation: number;
  runtime_phase: string;
  runtime_publication: 'published' | 'unchanged';
  first_owner_still_suppressed: boolean | null;
  abort_before_expiry: boolean | null;
  trigger_wait_sequence: number;
  accepted_wait_sequence: number;
  trigger_before_deadline: boolean;
  game_lease_available: boolean;
  operation_lease_available: boolean;
}

interface WatcherVerificationReport {
  complete: boolean;
  cases: WatcherVerificationCase[];
  error: string | null;
}

describe('Native watcher suppression verification', () => {
  let game: MockGame | undefined;
  let gameId: string;

  before(async () => {
    expect(await invokeInApp<string>('plugin:app|identifier')).toBe('com.reynalivan.emmm.e2e');
    const fixtureRoot = process.env.EMMM_WATCHER_VERIFICATION_ROOT;
    const nonce = process.env.EMMM_WATCHER_VERIFICATION_NONCE;
    if (!fixtureRoot || !nonce) {
      throw new Error(
        'Watcher verification config did not provide its owned fixture root and nonce',
      );
    }
    const expectedRoot = path.join(os.tmpdir(), `EMMM_WatcherVerification_${nonce}`);
    if (path.resolve(fixtureRoot) !== path.resolve(expectedRoot)) {
      throw new Error(
        `Watcher verification fixture root is not the expected temp child: ${fixtureRoot}`,
      );
    }

    game = await createMockGame('WatcherVerification', fixtureRoot);
    await Promise.all([
      addMockMod(game, 'ProbeObject', 'ReleaseSource'),
      addMockMod(game, 'ProbeObject', 'ExpirySource'),
      addMockMod(game, 'ProbeObject', 'AbortSource'),
    ]);
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(() => {
    if (game) scheduleMockGameRemoval(game);
  });

  it('proves native release, expiry, and abort suppression through the real event loop', async () => {
    if (!game) throw new Error('Watcher verification fixture was not initialized');
    const dashboard = await $('[data-testid="dashboard-layout"]');
    await dashboard.waitForDisplayed({ timeout: 30_000 });
    await fs.writeFile(path.join(game.root, '.watcher-verification-ready'), game.cleanupToken, {
      flag: 'wx',
    });

    const reportPath = path.join(game.root, 'watcher-verification.json');
    let report: WatcherVerificationReport | undefined;
    await browser.waitUntil(
      async () => {
        try {
          report = JSON.parse(await fs.readFile(reportPath, 'utf8')) as WatcherVerificationReport;
        } catch (error: unknown) {
          if ((error as NodeJS.ErrnoException).code === 'ENOENT') return false;
          throw error;
        }
        return report.complete;
      },
      {
        timeout: 60_000,
        interval: 100,
        timeoutMsg: 'Native watcher verification report did not complete within 60 seconds',
      },
    );

    if (!report) throw new Error('Native watcher verification completed without a report');
    const artifactDirectory = new URL(
      '../../../.audit-output/watcher-gap-verification/',
      import.meta.url,
    );
    await fs.mkdir(artifactDirectory, { recursive: true });
    await fs.writeFile(
      new URL('native-report.json', artifactDirectory),
      JSON.stringify(report, null, 2),
    );
    if (report.error) throw new Error(`Native watcher verification failed: ${report.error}`);
    expect(report.error).toBeNull();
    expect(report.cases.map((testCase) => testCase.name).sort()).toEqual([...CASE_NAMES].sort());
    for (const testCase of report.cases) {
      expect(testCase.accepted_revision).toBeGreaterThan(testCase.before_revision);
      expect(testCase.accepted_revision).toBe(testCase.reconcile_event_revision);
      expect(testCase.runtime_generation).toBeGreaterThan(0);
      expect(['succeeded', 'needs_manual_reload']).toContain(testCase.runtime_phase);
      expect(['published', 'unchanged']).toContain(testCase.runtime_publication);
      if (testCase.name === 'last_owner_release') {
        expect(testCase.first_owner_still_suppressed).toBe(true);
      }
      if (testCase.name === 'echo_abort') expect(testCase.abort_before_expiry).toBe(true);
      expect(testCase.accepted_wait_sequence).toBeGreaterThan(testCase.trigger_wait_sequence);
      expect(testCase.trigger_before_deadline).toBe(true);
      expect(testCase.game_lease_available).toBe(true);
      expect(testCase.operation_lease_available).toBe(true);
    }

    const probe = (await getObjects(gameId)).find((object) => object.name === 'ProbeObject');
    expect(probe?.mod_count).toBe(3);
    expect(probe?.enabled_count).toBe(1);
  });
});
