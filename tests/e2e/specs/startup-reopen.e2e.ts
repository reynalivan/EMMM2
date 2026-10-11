import { browser, $, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { randomUUID } from 'node:crypto';
import { BaseDirectory } from '@tauri-apps/api/path';
import path from 'node:path';
import {
  addMockMods,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { seedGameAndOpenDashboard } from '../support/app.js';
import { getObjects, waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';
import type { ActiveKeyBinding, AppSettings } from '../../../src/shared/api/tauri/bindings.gen.js';

const E2E_IDENTIFIER = 'com.reynalivan.emmm.e2e';

describe('Startup after onboarding and native process restart', () => {
  let game: MockGame;
  let gameId: string;
  let logPath: string;
  let stagingRoot: string;

  before(async () => {
    expect(await invokeInApp<string>('plugin:app|identifier')).toBe(E2E_IDENTIFIER);
    const localAppData = process.env.LOCALAPPDATA;
    if (!localAppData) throw new Error('LOCALAPPDATA is required for native startup diagnostics');
    logPath = path.join(localAppData, E2E_IDENTIFIER, 'logs', 'emmm.log');
    const appData = await invokeInApp<string>('plugin:path|resolve_directory', {
      directory: BaseDirectory.AppData,
    });
    if (path.basename(path.normalize(appData)) !== E2E_IDENTIFIER) {
      throw new Error('Refusing to create a staging fixture outside the verified E2E app data');
    }
    stagingRoot = path.join(appData, 'import-staging');
    game = await createMockGame('StartupReopen');
    await addMockMods(
      game,
      'StartupObject',
      Array.from({ length: 300 }, (_, index) => `Mod${index}`),
    );
    await fs.writeFile(
      path.join(game.modsPath, 'StartupObject', 'Mod0', 'keys.ini'),
      '[KeyStartupBackground]\nkey = F1\n',
    );
    await addMockMods(game, 'StartupObject', ['DISABLED Variants/HiddenMod']);
    await fs.writeFile(
      path.join(game.modsPath, 'StartupObject', 'DISABLED Variants', 'HiddenMod', 'keys.ini'),
      '[KeyDisabledAncestor]\nkey = F12\n',
    );
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(() => {
    if (game) scheduleMockGameRemoval(game);
  });

  it('reopens three times with one baseline and one watcher per native process', async () => {
    for (let run = 1; run <= 3; run += 1) {
      const orphanAttempt = path.join(stagingRoot, randomUUID(), randomUUID(), randomUUID());
      await fs.mkdir(orphanAttempt, { recursive: true });
      await fs.writeFile(path.join(orphanAttempt, 'owned-e2e-payload.txt'), 'startup cleanup');
      const previousLog = await fs.readFile(logPath, 'utf8');
      await browser.reloadSession();
      await browser.url('http://tauri.localhost/');
      await browser.waitUntil(
        () =>
          browser.execute(() => {
            const scope = window as unknown as {
              __TAURI__?: { core?: { invoke?: unknown } };
            };
            return typeof scope.__TAURI__?.core?.invoke === 'function';
          }),
        { timeout: 20_000, timeoutMsg: 'Fresh native process did not expose its IPC bridge' },
      );
      expect(await invokeInApp<string>('plugin:app|identifier')).toBe(E2E_IDENTIFIER);
      const dashboard = await $('[data-testid="dashboard-layout"]');
      await dashboard.waitForDisplayed({ timeout: 20_000 });
      const shellObservedMs = await browser.execute(() => performance.now());
      const settings = await invokeInApp<AppSettings>('get_settings');
      expect(settings.active_game_id).toBe(gameId);
      await waitForWorkspaceCoreReady(gameId);
      try {
        await browser.waitUntil(
          async () => {
            for (const key of await browser.$$('kbd=F1')) {
              if (await key.isDisplayed()) return true;
            }
            return false;
          },
          { timeout: 20_000, timeoutMsg: 'Background keybinding scan did not appear in Home' },
        );
      } catch (error) {
        const bindings = await invokeInApp<ActiveKeyBinding[]>('get_active_keybindings', {
          gameId,
        });
        const state = await browser.execute(() => ({
          statuses: [...document.querySelectorAll('[role="status"], [role="alert"]')].map(
            (element) => element.textContent,
          ),
          keys: [...document.querySelectorAll('kbd')].map((element) => element.textContent),
        }));
        console.info(`startup-keybinding diagnostics=${JSON.stringify({ bindings, state })}`);
        throw error;
      }
      await browser.waitUntil(
        async () => {
          const log = await fs.readFile(logPath, 'utf8');
          return log.slice(previousLog.length).includes('background: task log cleanup completed');
        },
        { timeout: 20_000, timeoutMsg: 'Deferred task-log retention did not finish' },
      );
      await browser.waitUntil(() => !existsSync(orphanAttempt), {
        timeout: 20_000,
        timeoutMsg: 'Startup did not detach the isolated orphan staging attempt',
      });
      await browser.waitUntil(() => !existsSync(path.join(stagingRoot, '.cleanup')), {
        timeout: 20_000,
        timeoutMsg: 'Native Ready worker did not finish deferred staging cleanup',
      });
      const bindings = await invokeInApp<ActiveKeyBinding[]>('get_active_keybindings', { gameId });
      expect(bindings.some((binding) => binding.key === 'F1')).toBe(true);
      expect(bindings.some((binding) => binding.key === 'F12')).toBe(false);
      for (const key of await browser.$$('kbd=F12')) {
        expect(await key.isDisplayed()).toBe(false);
      }
      const objects = await getObjects(gameId);
      const startupObject = objects.find((object) => object.name === 'StartupObject');
      expect(startupObject?.mod_count).toBe(301);

      const currentLog = await fs.readFile(logPath, 'utf8');
      if (!currentLog.startsWith(previousLog)) {
        throw new Error('Startup log rotated during bounded restart measurement');
      }
      const restartLog = currentLog.slice(previousLog.length);
      expect(restartLog).toContain('startup: detached 1 staging payload(s) for deferred cleanup');
      expect(restartLog).toContain('ready: removed 1 detached staging payload(s)');
      const registrations = restartLog
        .split('\n')
        .filter((line) => line.includes('watcher registration session='));
      const baselines = restartLog
        .split('\n')
        .filter(
          (line) =>
            line.includes(`disk reconcile timing game_id=${gameId} `) &&
            line.includes('reason=StartupBoot') &&
            line.includes('scan_scope=Full'),
        );
      const duplicateActivationScans = restartLog
        .split('\n')
        .filter(
          (line) =>
            line.includes(`disk reconcile timing game_id=${gameId} `) &&
            line.includes('reason=GameSwitched') &&
            line.includes('scan_scope=Full'),
        );
      expect(registrations).toHaveLength(1);
      expect(baselines).toHaveLength(1);
      expect(duplicateActivationScans).toHaveLength(0);
      const taskCleanup = restartLog
        .split('\n')
        .filter((line) => line.includes('background: task log cleanup completed'));
      expect(taskCleanup).toHaveLength(1);
      expect(restartLog.indexOf(taskCleanup[0])).toBeGreaterThan(restartLog.indexOf(baselines[0]));
      expect(restartLog).not.toContain('old task log(s) before boot reconcile');
      console.info(
        `startup-reopen run=${run} fixture_mods=${startupObject?.mod_count} shell_observed_page_ms=${Math.round(shellObservedMs)} watcher_installs=${registrations.length} startup_baselines=${baselines.length}`,
      );
    }
  });
});
