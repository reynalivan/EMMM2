import { $, browser, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import type { CollectionSummary } from '../../../src/shared/api/tauri/bindings.gen.js';
import { gotoWorkspaceView, seedGameAndOpenDashboard } from '../support/app.js';
import { createObject, reconcile } from '../support/data.js';
import { addMockMod, createMockGame, scheduleMockGameRemoval } from '../support/fixtures.js';
import { invokeInApp } from '../support/ipc.js';

describe('Collection dialogs support native Escape dismissal', () => {
  const output = path.resolve('.audit-output/fix-collections-apply-20261017/native');
  let gameId: string;
  let saved: CollectionSummary;

  before(async () => {
    await fs.mkdir(output, { recursive: true });
    const game = await createMockGame('CollectionDialogDismissal');
    scheduleMockGameRemoval(game);
    gameId = await seedGameAndOpenDashboard(game);
    await createObject(gameId, 'DialogObject');
    await addMockMod(game, 'DialogObject', 'DialogMod');
    await reconcile(gameId);
    saved = await invokeInApp<CollectionSummary>('create_collection', {
      gameId,
      name: 'Escape target',
      saveMode: 'save_current_state',
    });
    await invokeInApp('create_collection', {
      gameId,
      name: 'Active baseline',
      saveMode: 'save_current_state',
    });
    await browser.url('http://tauri.localhost/');
    await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: 25_000 });
    await gotoWorkspaceView('collections');
  });

  async function capture(name: string): Promise<void> {
    await browser.saveScreenshot(path.join(output, `${name}.png`));
    const dialogs = await browser.execute(() =>
      [...document.querySelectorAll('dialog')].map((dialog) => ({
        open: dialog.open,
        modal: dialog.matches(':modal'),
        text: dialog.innerText,
      })),
    );
    await fs.writeFile(path.join(output, `${name}.json`), JSON.stringify(dialogs, null, 2));
  }

  it('dismisses loaded Apply with Escape and can reopen it', async () => {
    for (let attempt = 0; attempt < 2; attempt++) {
      const row = await $('tr*=Escape target');
      await row.waitForDisplayed();
      await row.$('button*=Apply').click();
      const confirm = await $('[data-testid="modal-apply-btn"]');
      await confirm.waitForDisplayed();
      await browser.waitUntil(() => confirm.isEnabled(), { timeout: 15_000 });
      await $('dialog:modal').waitForDisplayed();
      await capture(`apply-${attempt}-before`);
      await browser.keys('Escape');
      await $('dialog:modal').waitForExist({ reverse: true });
      await confirm.waitForExist({ reverse: true });
      await capture(`apply-${attempt}-after`);
    }
  });

  it('dismisses Delete with Escape, reopens, and preserves the collection', async () => {
    for (let attempt = 0; attempt < 2; attempt++) {
      await $('tr*=Escape target').$('button[title="Delete collection"]').click();
      await $('dialog:modal').waitForDisplayed();
      await capture(`delete-${attempt}-before`);
      await browser.keys('Escape');
      await $('dialog:modal').waitForExist({ reverse: true });
      await capture(`delete-${attempt}-after`);
      const collections = await invokeInApp<CollectionSummary[]>('list_collections', { gameId });
      expect(collections.some((collection) => collection.id === saved.id)).toBe(true);
    }
  });
});
