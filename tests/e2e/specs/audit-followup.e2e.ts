import { browser, $, $$, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import {
  addMockMod,
  createMockGame,
  listDir,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { seedGameAndOpenDashboard, gotoWorkspaceView } from '../support/app.js';
import { createObject, reconcile, waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';
import type { ImportBatch, ImportBatchReport } from '../../../src/shared/api/tauri/bindings.gen.js';

async function visibleElement(selector: string) {
  let found: WebdriverIO.Element | undefined;
  await browser.waitUntil(
    async () => {
      for (const element of await $$(selector)) {
        if (await element.isDisplayed()) {
          found = element;
          return true;
        }
      }
      return false;
    },
    { timeout: 10_000, timeoutMsg: `Visible element not found: ${selector}` },
  );
  if (!found) throw new Error(`Visible element not found: ${selector}`);
  return found;
}

async function visibleButton(text: string) {
  let found: WebdriverIO.Element | undefined;
  await browser.waitUntil(
    async () => {
      for (const button of await $$('button')) {
        if ((await button.isDisplayed()) && (await button.getText()) === text) {
          found = button;
          return true;
        }
      }
      return false;
    },
    { timeout: 10_000, timeoutMsg: `Visible button not found: ${text}` },
  );
  if (!found) throw new Error(`Visible button not found: ${text}`);
  return found;
}

describe('Audit follow-up — native import and randomizer regressions', () => {
  let game: MockGame;
  let gameId: string;
  let objectA: string;
  let objectB: string;

  before(async () => {
    game = await createMockGame('AuditFollowup');
    gameId = await seedGameAndOpenDashboard(game);
    objectA = await createObject(gameId, 'AuditA', 'Character');
    objectB = await createObject(gameId, 'AuditB', 'Character');
    for (const [object, hash] of [
      ['AuditA', 'aaaabbbbccccdddd'],
      ['AuditB', '1111222233334444'],
    ]) {
      for (const name of ['Current', 'DISABLED Alternative']) {
        const folder = await addMockMod(game, object, name);
        await fs.writeFile(
          path.join(folder, 'mod.ini'),
          `[TextureOverrideAudit]\nhash = ${hash}\n`,
        );
      }
    }
    await fs.mkdir(path.join(game.modsPath, 'AuditA', 'Variants'));
    await reconcile(gameId);
  });

  beforeEach(async () => {
    await browser.url('http://tauri.localhost/');
    await $('[data-testid="dashboard-layout"]').waitForExist({ timeout: 25_000 });
    await waitForWorkspaceCoreReady(gameId);
    await gotoWorkspaceView('mods');
    await browser.waitUntil(
      async () => {
        for (const element of await $$(`[data-object-id="${objectA}"]`)) {
          if (await element.isDisplayed()) return true;
        }
        return false;
      },
      { timeout: 10_000, timeoutMsg: 'Object sidebar did not render the owned fixture' },
    );
    await (await visibleElement(`[data-object-id="${objectA}"]`)).click();
    await (await visibleElement('[data-testid="folder-grid"]')).waitForDisplayed();
  });

  afterEach(async function () {
    if (this.currentTest?.state === 'failed') {
      const diagnostic = await browser.execute(() => {
        return {
          title: document.querySelector('dialog[open] h3')?.textContent,
          text: document.querySelector('dialog[open]')?.textContent,
          viewport: { width: innerWidth, height: innerHeight },
          buttons: [...document.querySelectorAll<HTMLButtonElement>('dialog[open] button')].map(
            (button) => {
              const rect = button.getBoundingClientRect();
              return {
                text: button.textContent,
                disabled: button.disabled,
                bounds: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
              };
            },
          ),
        };
      });
      const output = path.resolve('.audit-output/audit-followup-20261011');
      const name = this.currentTest.title.includes('Keep current')
        ? 'keep-current'
        : 'pending-preview';
      await fs.writeFile(
        path.join(output, `${name}-diagnostic.json`),
        JSON.stringify(diagnostic, null, 2),
      );
      console.info('Audit native diagnostic:', JSON.stringify(diagnostic));
    }
  });

  after(() => scheduleMockGameRemoval(game));

  async function incoming(name: string): Promise<string> {
    const source = path.join(game.root, 'incoming', name);
    await fs.mkdir(source, { recursive: true });
    await fs.writeFile(
      path.join(source, 'mod.ini'),
      '[TextureOverrideIncoming]\nhash = 56781234\n',
    );
    return source;
  }

  async function nativeDrop(selector: string, source: string): Promise<ImportBatch> {
    const before = await invokeInApp<ImportBatch[]>('list_import_batches', { gameId });
    const position = await browser.execute((targetSelector: string) => {
      const target = [...document.querySelectorAll(targetSelector)].find((element) => {
        const bounds = element.getBoundingClientRect();
        return bounds.width > 0 && bounds.height > 0;
      });
      if (!target) throw new Error(`Missing native drop target ${targetSelector}`);
      const rect = target.getBoundingClientRect();
      if (rect.width <= 0 || rect.height <= 0) throw new Error('Native drop target is hidden');
      return {
        x: (rect.left + rect.width / 2) * devicePixelRatio,
        y: (rect.top + rect.height / 2) * devicePixelRatio,
      };
    }, selector);
    // Inject through Tauri's real event IPC; this certifies routing, not an OS drag gesture.
    await invokeInApp('plugin:event|emit', {
      event: 'tauri://drag-drop',
      payload: { paths: [source], position },
    });
    let created: ImportBatch[] = [];
    await browser.waitUntil(
      async () => {
        const batches = await invokeInApp<ImportBatch[]>('list_import_batches', { gameId });
        created = batches.filter((batch) => !before.some((old) => old.id === batch.id));
        return created.length > 0 && created.every((batch) => batch.status === 'awaiting_review');
      },
      { timeout: 15_000, interval: 100, timeoutMsg: 'Native drop did not finish analysis' },
    );
    expect(created).toHaveLength(1);
    expect(created[0].status).toBe('awaiting_review');
    return created[0];
  }

  it('a sidebar drop launches exactly one batch for the dropped object', async () => {
    const batch = await nativeDrop(
      `[data-object-id="${objectB}"]`,
      await incoming('SidebarSource'),
    );
    expect(batch.targetObjectId).toBe(objectB);
    expect(batch.targetSubpath).toBeNull();
    await invokeInApp('cancel_import_batch', { batchId: batch.id });
  });

  it('a nested grid drop commits into its relative subfolder without duplicating the object path', async () => {
    const variants = await visibleElement('[title="Variants"]');
    await variants.waitForDisplayed({ timeout: 10_000 });
    await variants.doubleClick();
    const source = await incoming('GridSource');
    const batch = await nativeDrop('[data-testid="folder-grid"]', source);
    expect(batch.targetObjectId).toBe(objectA);
    expect(batch.targetSubpath).toBe('Variants');
    const item = batch.items[0];
    if (item.status === 'awaiting_category') {
      await invokeInApp('set_import_item_classification', {
        input: {
          itemId: item.id,
          category: 'Other',
          subCategory: null,
          metadata: {},
        },
      });
    }
    await invokeInApp('refresh_import_item_suggestions', { itemId: item.id });
    await invokeInApp('set_import_item_decision', {
      input: {
        itemId: item.id,
        decision: 'keep_specific_target',
        destinationObjectId: objectA,
        destinationPath: null,
        canonicalEntryKey: null,
        matchedAlias: null,
      },
    });
    await invokeInApp('mark_import_batch_review_started', { batchId: batch.id });
    const report = await invokeInApp<ImportBatchReport>('commit_import_batch', {
      input: {
        batchId: batch.id,
        itemIds: [item.id],
      },
    });
    expect(report.moved).toBe(1);
    await waitForWorkspaceCoreReady(gameId);
    expect(await listDir(path.join(game.modsPath, 'AuditA', 'Variants'))).toContain(
      'DISABLED GridSource',
    );
    expect(
      await fs.stat(
        path.join(game.modsPath, 'AuditA', 'Variants', 'DISABLED GridSource', 'mod.ini'),
      ),
    ).toBeDefined();
  });

  it('validation failure settles the native batch and allows retry after repairing its source', async () => {
    const source = await incoming('RetrySource');
    const rejectedFile = path.join(source, 'unsafe.exe');
    await fs.writeFile(rejectedFile, 'owned inert fixture');
    const batch = await invokeInApp<ImportBatch>('create_import_batch', {
      input: {
        gameId,
        flow: 'specific_import',
        targetMode: 'specific',
        targetObjectId: objectB,
        targetSubpath: null,
        sources: [{ path: source, sourceKind: 'folder' }],
      },
    });
    await expect(invokeInApp('analyze_import_batch', { batchId: batch.id })).rejects.toThrow();
    const failed = await invokeInApp<ImportBatch>('get_import_batch', { batchId: batch.id });
    expect(failed.status).toBe('partial');
    expect(failed.items[0].status).toBe('failed');
    await fs.unlink(rejectedFile);
    const retried = await invokeInApp<ImportBatch>('analyze_import_batch', { batchId: batch.id });
    expect(retried.status).toBe('awaiting_review');
    expect(retried.items[0].status).not.toBe('failed');
    await invokeInApp('cancel_import_batch', { batchId: batch.id });
  });

  async function roll() {
    await $('button[title="Randomizer (Gacha)"]').click();
    await $('dialog[open]').waitForDisplayed();
    await (await visibleButton('Roll Luck')).click();
    await browser.waitUntil(
      () =>
        browser.execute(() =>
          [...document.querySelectorAll('dialog[open] button')].some((button) =>
            button.textContent?.includes('Review changes (2)'),
          ),
        ),
      { timeout: 10_000, timeoutMsg: 'Randomizer did not produce two recommendations' },
    );
  }

  it('Keep current survives row clicks and Select All, and apply preserves that object on disk', async () => {
    await roll();
    const keepButtons = await $$('dialog[open] button');
    for (const button of keepButtons) {
      if (
        (await button.getText()) === 'Keep current' &&
        /\baudita\b/i.test(await (await button.$('..')).getText())
      ) {
        await button.click();
        break;
      }
    }
    const kept = await visibleButton('Keeping current');
    const keptRow = await kept.$('..');
    await expect(keptRow).toHaveText(/\baudita\b/i);
    await keptRow.click({ x: 10, y: 10 });
    await (await visibleButton('Deselect All')).click();
    await (await visibleButton('Select All')).click();
    expect(await (await visibleButton('Keeping current')).isDisplayed()).toBe(true);
    const review = await visibleButton('Review changes (1)');
    await review.waitForEnabled({ timeout: 10_000 });
    await review.click();
    await browser.waitUntil(
      async () => (await (await $$('dialog[open] section li')).length) === 1,
      {
        timeout: 10_000,
        timeoutMsg: 'Locked object was included in the apply preview',
      },
    );
    const warning = await $('dialog[open] section label input[type="checkbox"]');
    if (await warning.isExisting()) await warning.click();
    await (await visibleButton('Confirm & Apply (1)')).click();
    await waitForWorkspaceCoreReady(gameId);
    expect(await listDir(path.join(game.modsPath, 'AuditA'))).toEqual(
      expect.arrayContaining(['Current', 'DISABLED Alternative']),
    );
  });

  it('changing selection invalidates a reviewed loadout and a fresh native review settles', async () => {
    await roll();
    const review = await visibleButton('Review changes (2)');
    await review.waitForEnabled({ timeout: 10_000 });
    await review.click();
    await browser.waitUntil(async () => (await (await $$('dialog[open] section li')).length) === 2);
    await (await visibleButton('Deselect All')).click();
    await (await visibleButton('Select All')).click();
    await expect(await visibleButton('Review changes (2)')).toBeEnabled();
    expect(await (await $$('dialog[open] section')).length).toBe(0);
    await (await visibleButton('Review changes (2)')).click();
    await browser.waitUntil(
      async () => (await (await $$('dialog[open] section li')).length) === 2,
      {
        timeout: 10_000,
        timeoutMsg: 'A fresh review did not settle after the old response',
      },
    );
  });

  it('saving the current loadout through the UI refreshes the visible collection list', async () => {
    await gotoWorkspaceView('collections');
    await (await visibleButton('Save Current State')).click();
    const input = await $('dialog[open] input[placeholder]');
    await input.waitForDisplayed();
    await input.setValue('Audit visible saved collection');
    await $('dialog[open] button[type="submit"]').click();
    await browser.waitUntil(
      () =>
        browser.execute(
          () =>
            !document.querySelector('dialog[open]') &&
            document.body.textContent?.includes('Audit visible saved collection') === true,
        ),
      { timeout: 10_000, timeoutMsg: 'Saved collection was not rendered after mutation refresh' },
    );
    const collections = await invokeInApp<Array<{ name: string }>>('list_collections', { gameId });
    expect(
      collections.some((collection) => collection.name === 'Audit visible saved collection'),
    ).toBe(true);
  });
});
