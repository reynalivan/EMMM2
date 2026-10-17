import { $, browser, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import { gotoWorkspaceView, seedGameAndOpenDashboard } from '../support/app.js';
import { createObject, reconcile, waitForWorkspaceCoreReady } from '../support/data.js';
import {
  addMockMod,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { invokeInApp } from '../support/ipc.js';

describe('Conflict Review remains reachable with ignored groups', function () {
  this.timeout(90_000);
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('ConflictIgnoredEntry');
    scheduleMockGameRemoval(game);
    gameId = await seedGameAndOpenDashboard(game);
    await createObject(gameId, 'ConflictEntry', 'Character');
    for (const name of ['VariantOne', 'VariantTwo']) {
      const mod = await addMockMod(game, 'ConflictEntry', name);
      await fs.writeFile(path.join(mod, 'mod.ini'), '[TextureOverrideEntry]\nhash = aabbccdd\n');
    }
    await reconcile(gameId);
    await browser.setWindowSize(1360, 900);
  });

  async function openReview(unresolved: number): Promise<void> {
    const entry = await $(`button[aria-label="Shared hashes: ${unresolved}"]`);
    await entry.waitForClickable({ timeout: 20_000 });
    await entry.click();
    await $('dialog[open]').waitForDisplayed();
  }

  async function closeReview(): Promise<void> {
    const close = await $('dialog[open] button[aria-label="Close"]');
    await close.waitForClickable();
    await close.click();
    await $('dialog[open]').waitForDisplayed({ reverse: true });
  }

  async function reloadToMods(restart: boolean): Promise<void> {
    if (restart) await browser.reloadSession();
    await browser.url('http://tauri.localhost/');
    await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: 25_000 });
    expect(await invokeInApp<string>('plugin:app|identifier')).toBe('com.reynalivan.emmm.e2e');
    await waitForWorkspaceCoreReady(gameId);
    if (restart) {
      const report = await invokeInApp<{ error_code: string } | null>('get_pending_crash_report');
      if (report) {
        expect(report.error_code).toBe('unknown');
        const notification = await $('dialog[open][aria-labelledby="crash-report-title"]');
        await notification.waitForDisplayed();
        await notification.$('button=Remove report').click();
        await notification.waitForExist({ reverse: true });
        await browser.waitUntil(
          async () => (await invokeInApp<unknown>('get_pending_crash_report')) === null,
          { timeout: 15_000, timeoutMsg: 'Private restart report was not removed' },
        );
      }
    }
    await gotoWorkspaceView('mods');
  }

  it('reopens ignored groups after reload/restart and restores them through the UI', async () => {
    await gotoWorkspaceView('mods');
    await openReview(1);
    await $('dialog[open]').$('button=Ignore group').click();
    await $('dialog[open]').$('button=Ignored (1)').waitForDisplayed();
    await browser.waitUntil(
      async () =>
        (await invokeInApp<string[]>('list_ignored_active_mod_conflict_group_keys', { gameId }))
          .length === 1,
      { timeout: 15_000, timeoutMsg: 'Ignored conflict group did not persist' },
    );
    await closeReview();

    for (const restart of [false, true]) {
      await reloadToMods(restart);
      await openReview(0);
      await $('dialog[open]').$('button=Ignored (1)').click();
      await $('dialog[open]').$('button=Restore group').waitForClickable();
      await closeReview();
    }

    await openReview(0);
    await $('dialog[open]').$('button=Ignored (1)').click();
    await $('dialog[open]').$('button=Restore group').click();
    await browser.waitUntil(
      async () =>
        (await invokeInApp<string[]>('list_ignored_active_mod_conflict_group_keys', { gameId }))
          .length === 0,
      { timeout: 15_000, timeoutMsg: 'Restored conflict group remained ignored' },
    );
    await $('dialog[open]').$('button=Unresolved (1)').waitForDisplayed();
    await closeReview();
    await reloadToMods(false);
    await $('button[aria-label="Shared hashes: 1"]').waitForDisplayed({ timeout: 20_000 });
    for (const name of ['VariantOne', 'VariantTwo']) {
      expect((await fs.stat(path.join(game.modsPath, 'ConflictEntry', name))).isDirectory()).toBe(
        true,
      );
    }
  });
});
