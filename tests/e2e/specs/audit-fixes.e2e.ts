import { $, browser, expect } from '@wdio/globals';
import { BaseDirectory } from '@tauri-apps/api/path';
import { randomUUID } from 'node:crypto';
import fs from 'node:fs/promises';
import { createServer, type Server } from 'node:http';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import type {
  PendingCrashReportSummary,
  PipelineTask,
} from '../../../src/shared/api/tauri/bindings.gen.js';
import { gotoWorkspaceView, seedGameAndOpenDashboard } from '../support/app.js';
import { waitForWorkspaceCoreReady } from '../support/data.js';
import { createMockGame, scheduleMockGameRemoval, type MockGame } from '../support/fixtures.js';
import { invokeInApp } from '../support/ipc.js';

const E2E_IDENTIFIER = 'com.reynalivan.emmm.e2e';
const WAIT_TIMEOUT_MS = 30_000;
const DATABASE_BUSY_TIMEOUT_MS = 2_500;

describe('Native audit fixes: startup recovery and browser schemes', function () {
  this.timeout(120_000);
  let game: MockGame;
  let gameId: string;
  let databasePath: string;
  let server: Server;
  let origin: string;
  const taskId = randomUUID();
  const requests = new Set<string>();

  function withFixtureDatabase<T>(operation: (database: DatabaseSync) => T): T {
    const database = new DatabaseSync(databasePath, { allowExtension: false });
    try {
      database.exec(`PRAGMA busy_timeout = ${DATABASE_BUSY_TIMEOUT_MS}`);
      if (database.prepare('SELECT id FROM games WHERE id = ?').get(gameId)?.id !== gameId) {
        throw new Error('Owned fixture game is absent from the verified E2E database');
      }
      return operation(database);
    } finally {
      database.close();
    }
  }

  before(async () => {
    expect(await invokeInApp<string>('plugin:app|identifier')).toBe(E2E_IDENTIFIER);
    game = await createMockGame('AuditFixes');
    gameId = await seedGameAndOpenDashboard(game);
    const appData = await invokeInApp<string>('plugin:path|resolve_directory', {
      directory: BaseDirectory.AppData,
    });
    const appDataRoot = await fs.realpath(appData);
    if (path.basename(appDataRoot) !== E2E_IDENTIFIER) {
      throw new Error('Refusing recovery fixture outside the private E2E app data');
    }
    const requestedPath = path.join(appDataRoot, 'app.db');
    const databaseStat = await fs.lstat(requestedPath);
    if (!databaseStat.isFile() || databaseStat.isSymbolicLink()) {
      throw new Error('Recovery fixture database must be an existing regular file');
    }
    databasePath = await fs.realpath(requestedPath);
    if (path.dirname(databasePath) !== appDataRoot) {
      throw new Error('Recovery fixture database escaped the verified E2E app data');
    }
    server = createServer((request, response) => {
      requests.add(request.url ?? '');
      response.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      response.end('<title>Audit URL fixture</title><p>Owned local fixture</p>');
    });
    await new Promise<void>((resolve, reject) => {
      server.once('error', reject);
      server.listen(0, '127.0.0.1', resolve);
    });
    const address = server.address();
    if (!address || typeof address === 'string') throw new Error('Local URL fixture did not bind');
    origin = `http://127.0.0.1:${address.port}`;
  });

  after(async () => {
    if (game) scheduleMockGameRemoval(game);
    try {
      if (databasePath && gameId) {
        withFixtureDatabase((database) => {
          database.prepare('DELETE FROM tasks WHERE id = ? AND game_id = ?').run(taskId, gameId);
        });
      }
    } finally {
      if (server?.listening) {
        await new Promise<void>((resolve, reject) => {
          server.close((error) => (error ? reject(error) : resolve()));
          server.closeAllConnections();
        });
      }
    }
  });

  it('dismisses the splash for a real recovery task and returns to the dashboard', async () => {
    expect(await invokeInApp<PipelineTask[]>('app_startup_check')).toHaveLength(0);
    withFixtureDatabase((database) => {
      database
        .prepare('INSERT INTO tasks (id, game_id, task_type, status) VALUES (?, ?, ?, ?)')
        .run(taskId, gameId, 'apply_collection', 'PENDING');
    });
    await browser.reloadSession();
    await browser.url('http://tauri.localhost/');
    await browser.waitUntil(
      () =>
        browser.execute(() => {
          const scope = window as Window & { __TAURI__?: { core?: { invoke?: unknown } } };
          return typeof scope.__TAURI__?.core?.invoke === 'function';
        }),
      {
        timeout: WAIT_TIMEOUT_MS,
        timeoutMsg: 'Restarted private app did not expose its IPC bridge',
      },
    );
    expect(await invokeInApp<string>('plugin:app|identifier')).toBe(E2E_IDENTIFIER);
    const recovery = await $('dialog[aria-labelledby="recovery-dialog-title"]');
    await recovery.waitForDisplayed({ timeout: WAIT_TIMEOUT_MS });
    await $('#splash').waitForExist({ reverse: true, timeout: WAIT_TIMEOUT_MS });
    const ignore = await recovery.$('button=Ignore Task');
    await ignore.waitForClickable({ timeout: WAIT_TIMEOUT_MS });
    await ignore.click();
    await recovery.waitForExist({ reverse: true, timeout: WAIT_TIMEOUT_MS });
    await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: WAIT_TIMEOUT_MS });
    await waitForWorkspaceCoreReady(gameId);
    expect(await $('#splash').isExisting()).toBe(false);
    expect(await invokeInApp<PipelineTask[]>('app_startup_check')).toHaveLength(0);
    expect(
      withFixtureDatabase((database) =>
        database
          .prepare('SELECT status FROM tasks WHERE id = ? AND game_id = ?')
          .get(taskId, gameId),
      )?.status,
    ).toBe('FAILED');
    const restartReport = await invokeInApp<PendingCrashReportSummary | null>(
      'get_pending_crash_report',
    );
    if (restartReport) {
      expect(restartReport.error_code).toBe('unknown');
      const notification = await $('dialog[open][aria-labelledby="crash-report-title"]');
      await notification.waitForDisplayed({ timeout: WAIT_TIMEOUT_MS });
      await notification.$('button=Remove report').click();
      await notification.waitForExist({ reverse: true, timeout: WAIT_TIMEOUT_MS });
      await browser.waitUntil(
        async () =>
          (await invokeInApp<PendingCrashReportSummary | null>('get_pending_crash_report')) ===
          null,
        { timeout: WAIT_TIMEOUT_MS, timeoutMsg: 'Private restart report was not removed' },
      );
    }
  });

  it('navigates uppercase HTTP from the address field to the intended local host', async () => {
    await gotoWorkspaceView('browser');
    const address = await $('#browser-url-input');
    await address.waitForClickable({ timeout: WAIT_TIMEOUT_MS });
    await address.click();
    const input = await $('input#browser-url-input');
    await input.waitForDisplayed({ timeout: WAIT_TIMEOUT_MS });
    const route = '/CaseSensitivePath?Key=Value';
    const url = `${origin.replace('http:', 'HTTP:')}${route}`;
    await browser.keys(['Control', 'a']);
    await browser.keys(['Backspace']);
    await browser.keys(Array.from(url));
    await expect(input).toHaveValue(url);
    await browser.keys('Enter');
    const confirmation = await $('dialog[open]');
    await confirmation.waitForDisplayed({ timeout: WAIT_TIMEOUT_MS });
    await expect(confirmation).toHaveText(/not using HTTPS/);
    const confirm = await confirmation.$('button=Confirm');
    await confirm.waitForClickable({ timeout: WAIT_TIMEOUT_MS });
    await confirm.click();
    await confirmation.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT_MS });
    await browser.waitUntil(() => requests.has(route), {
      timeout: WAIT_TIMEOUT_MS,
      timeoutMsg: 'Uppercase address did not reach the intended local HTTP server',
    });
  });

  it('recognizes a mixed-case scheme sent directly to native navigation', async () => {
    const route = '/NativeCaseSensitivePath?Key=Value';
    const label = await invokeInApp<string>('browser_open_tab', {
      url: `${origin.replace('http:', 'hTtP:')}${route}`,
      sessionId: null,
    });
    try {
      await browser.waitUntil(() => requests.has(route), {
        timeout: WAIT_TIMEOUT_MS,
        timeoutMsg: 'Native mixed-case scheme did not reach the local HTTP server',
      });
    } finally {
      await browser
        .executeAsync(async (tabLabel: string, done: (error: string | null) => void) => {
          try {
            const scope = window as unknown as Window & {
              __TAURI__: {
                webview: {
                  Webview: {
                    getByLabel: (value: string) => Promise<{ close: () => Promise<void> } | null>;
                  };
                };
              };
            };
            await (await scope.__TAURI__.webview.Webview.getByLabel(tabLabel))?.close();
            done(null);
          } catch (error: unknown) {
            done(error instanceof Error ? error.message : String(error));
          }
        }, label)
        .then((error) => {
          if (error !== null) throw new Error(`Owned native tab cleanup failed: ${error}`);
        });
    }
  });
});
