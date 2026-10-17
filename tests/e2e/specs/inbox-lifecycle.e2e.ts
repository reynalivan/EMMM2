import { $, $$, browser, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
import { createMockGame, scheduleMockGameRemoval, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard, gotoWorkspaceView } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import type {
  AppSettings,
  ImportBatch,
  ImportSourcePreview,
} from '../../../src/shared/api/tauri/bindings.gen.js';

const UI_TIMEOUT = 15_000;
const NOTICE = 'More files…';

interface HeldCompletion {
  entered: boolean;
  registrations: number;
  watcherCompletions: number;
  launchRegistrations: number;
  batchId: string | null;
  release: () => void;
  restore: () => void;
}
interface HoldWindow extends Window {
  __TAURI_INTERNALS__: { callbacks: Map<number, (response: unknown) => unknown> };
  __EMMM_INBOX_HOLD__?: HeldCompletion;
}

async function installCompletionHold(
  kind: 'batch' | 'watcher' | 'launch',
  gameId: string,
): Promise<void> {
  await browser.execute(
    (mode, ownedGameId) => {
      const target = window as unknown as HoldWindow;
      if (target.__EMMM_INBOX_HOLD__) throw new Error('Inbox completion hold already installed');
      const callbacks = target.__TAURI_INTERNALS__.callbacks;
      if (!(callbacks instanceof Map) || !Object.isExtensible(callbacks)) {
        throw new Error('Native callback map is unavailable');
      }
      const originalSet = callbacks.set;
      const originalDescriptor = Object.getOwnPropertyDescriptor(callbacks, 'set');
      const pending: Array<() => void> = [];
      let holding = true;
      const state: HeldCompletion = {
        entered: false,
        registrations: 0,
        watcherCompletions: 0,
        launchRegistrations: 0,
        batchId: null,
        release: () => {
          holding = false;
          for (const complete of pending.splice(0)) complete();
        },
        restore: () => {
          holding = false;
          if (originalDescriptor) Object.defineProperty(callbacks, 'set', originalDescriptor);
          else if (!Reflect.deleteProperty(callbacks, 'set')) {
            throw new Error('Cannot restore native callback registration');
          }
        },
      };
      target.__EMMM_INBOX_HOLD__ = state;
      Object.defineProperty(callbacks, 'set', {
        configurable: true,
        writable: true,
        value: (id: number, callback: (response: unknown) => unknown) => {
          const stack = new Error().stack ?? '';
          const watcherCallback = mode === 'watcher' && stack.includes('startModInboxWatcher');
          const launchCallback = stack.includes('getImportBatch');
          if (watcherCallback) state.registrations += 1;
          if (launchCallback) state.launchRegistrations += 1;
          return originalSet.call(callbacks, id, (response: unknown) => {
            const isBatch =
              mode !== 'watcher' &&
              typeof response === 'object' &&
              response !== null &&
              'gameId' in response &&
              response.gameId === ownedGameId &&
              'id' in response &&
              typeof response.id === 'string' &&
              'items' in response &&
              Array.isArray(response.items);
            const isWatcher = watcherCallback && (response === null || response === undefined);
            if (isBatch && typeof response.id === 'string') state.batchId = response.id;
            const complete = () => {
              if (isWatcher) state.watcherCompletions += 1;
              return callback(response);
            };
            if (
              holding &&
              ((mode === 'batch' && isBatch) ||
                isWatcher ||
                (mode === 'launch' && launchCallback && isBatch))
            ) {
              state.entered = true;
              pending.push(complete);
              return;
            }
            return complete();
          });
        },
      });
    },
    kind,
    gameId,
  );
}

async function releaseCompletionHold(): Promise<void> {
  await browser.execute(() => {
    (window as unknown as HoldWindow).__EMMM_INBOX_HOLD__?.release();
  });
}

async function restoreCompletionHold(): Promise<void> {
  await browser.execute(() => {
    const target = window as unknown as HoldWindow;
    target.__EMMM_INBOX_HOLD__?.release();
    target.__EMMM_INBOX_HOLD__?.restore();
    delete target.__EMMM_INBOX_HOLD__;
  });
}

async function waitForHeldCompletion(): Promise<void> {
  await browser.waitUntil(
    () =>
      browser.execute(
        () => (window as unknown as HoldWindow).__EMMM_INBOX_HOLD__?.entered === true,
      ),
    { timeout: UI_TIMEOUT, timeoutMsg: 'The actual native completion did not reach the hold' },
  );
}

describe('Native Inbox lifetime and bounded source preview', function () {
  this.timeout(90_000);
  let game: MockGame;
  let gameId: string;
  let inbox: string;
  let retiredBatchId: string;

  before(async () => {
    game = await createMockGame(
      'InboxLifecycle',
      path.join(os.tmpdir(), `EMMM_Inbox_${randomUUID()}`, 'GIMI'),
    );
    inbox = path.join(game.fixtureRoot, 'Inbox');
    await fs.mkdir(inbox);
    for (const name of ['FlatPack', 'DeepPack']) {
      const source = path.join(inbox, name);
      await fs.mkdir(source);
      await fs.writeFile(path.join(source, 'mod.ini'), '[TextureOverrideOwned]\nhash = faceb00c\n');
    }
    for (let index = 0; index < 13; index++) {
      await fs.writeFile(path.join(inbox, 'FlatPack', `payload-${index}.dat`), 'owned');
    }
    const hidden = path.join(inbox, 'DeepPack', 'A', 'B', 'C', 'D', 'E');
    await fs.mkdir(hidden, { recursive: true });
    await fs.writeFile(path.join(hidden, 'hidden.dat'), 'owned');
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

  after(async () => {
    await restoreCompletionHold();
    await browser.releaseActions();
    if (game) scheduleMockGameRemoval(game);
  });

  afterEach(async () => {
    await restoreCompletionHold();
    await browser.releaseActions();
  });

  it('does not launch a retired batch after real native completion', async () => {
    await gotoWorkspaceView('mod-inbox');
    const review = await $('button*=Review selected');
    await review.waitForClickable({ timeout: UI_TIMEOUT });
    await installCompletionHold('batch', gameId);
    await review.click();
    await waitForHeldCompletion();
    retiredBatchId = await browser.execute(() => {
      const id = (window as unknown as HoldWindow).__EMMM_INBOX_HOLD__?.batchId;
      if (!id) throw new Error('Held native batch omitted its identifier');
      return id;
    });
    await gotoWorkspaceView('dashboard');
    await releaseCompletionHold();
    const observationStarted = Date.now();
    await browser.waitUntil(
      async () => {
        if (await $('dialog[open]').isExisting()) throw new Error('Retired Inbox opened a dialog');
        if (await $('div[class*="workspace-layer-overlay"].grid.place-items-center').isExisting()) {
          throw new Error('Retired Inbox started wizard analysis');
        }
        if (
          await browser.execute(
            () =>
              ((window as unknown as HoldWindow).__EMMM_INBOX_HOLD__?.launchRegistrations ?? 0) > 0,
          )
        ) {
          throw new Error('Retired Inbox requested the wizard batch');
        }
        return Date.now() - observationStarted >= 1200;
      },
      { timeout: UI_TIMEOUT, interval: 50, timeoutMsg: 'Retired completion did not remain quiet' },
    );
    const batch = await invokeInApp<ImportBatch>('get_import_batch', { batchId: retiredBatchId });
    expect(batch.gameId).toBe(gameId);
    expect(batch.items.length).toBe(2);
  });

  it('preserves actual notifications after pending same-root watcher remount', async () => {
    await gotoWorkspaceView('dashboard');
    await installCompletionHold('watcher', gameId);
    await gotoWorkspaceView('mod-inbox');
    await waitForHeldCompletion();
    const originalRegistrations = await browser.execute(
      () => (window as unknown as HoldWindow).__EMMM_INBOX_HOLD__?.registrations ?? 0,
    );
    expect(originalRegistrations).toBeGreaterThan(0);
    await gotoWorkspaceView('dashboard');
    await gotoWorkspaceView('mod-inbox');
    await (await $('input[aria-label="Select FlatPack"]')).waitForExist({ timeout: UI_TIMEOUT });
    await browser.executeAsync((done: () => void) => {
      requestAnimationFrame(() => requestAnimationFrame(() => done()));
    });
    expect(
      await browser.execute(
        () => (window as unknown as HoldWindow).__EMMM_INBOX_HOLD__?.registrations ?? 0,
      ),
    ).toBe(originalRegistrations);
    await releaseCompletionHold();
    await browser.waitUntil(
      () =>
        browser.execute(
          () =>
            ((window as unknown as HoldWindow).__EMMM_INBOX_HOLD__?.watcherCompletions ?? 0) >= 2,
        ),
      { timeout: UI_TIMEOUT, timeoutMsg: 'Successor native watcher did not finish starting' },
    );
    const arriving = path.join(inbox, 'NotificationPack');
    await fs.mkdir(arriving);
    await fs.writeFile(path.join(arriving, 'mod.ini'), '[TextureOverrideOwned]\nhash = deadbeef\n');
    await (
      await $('input[aria-label="Select NotificationPack"]')
    ).waitForDisplayed({
      timeout: UI_TIMEOUT,
    });
  });

  it('discloses client and depth omissions in the real import preview', async () => {
    await invokeInApp('cancel_import_batch', { batchId: retiredBatchId });
    const refresh = await $('button[aria-label="Refresh"]');
    await refresh.waitForClickable({ timeout: UI_TIMEOUT });
    await refresh.click();
    for (const name of ['FlatPack', 'DeepPack']) {
      const source = await $(`input[type="checkbox"][aria-label="Select ${name}"]`);
      await source.waitForEnabled({ timeout: UI_TIMEOUT });
      if (!(await source.isSelected())) await source.click();
    }
    const review = await $('button*=Review selected');
    await review.waitForClickable({ timeout: UI_TIMEOUT });
    expect(await review.getText()).toContain('(2)');
    await installCompletionHold('launch', gameId);
    await review.click();
    await waitForHeldCompletion();
    const previewBatchId = await browser.execute(() => {
      const id = (window as unknown as HoldWindow).__EMMM_INBOX_HOLD__?.batchId;
      if (!id) throw new Error('Native preview batch omitted its identifier');
      return id;
    });
    expect(
      await browser.execute(
        () => (window as unknown as HoldWindow).__EMMM_INBOX_HOLD__?.launchRegistrations ?? 0,
      ),
    ).toBeGreaterThan(0);
    const analysis = await $('div[class*="workspace-layer-overlay"].grid.place-items-center');
    await analysis.waitForDisplayed({ timeout: UI_TIMEOUT });
    const controlStarted = Date.now();
    await browser.waitUntil(
      async () => {
        expect(await analysis.isDisplayed()).toBe(true);
        expect(await $('dialog[open]').isExisting()).toBe(false);
        return Date.now() - controlStarted >= 1500;
      },
      { timeout: UI_TIMEOUT, interval: 50 },
    );
    await releaseCompletionHold();
    const resume = await $('button=Continue review');
    if (await resume.isDisplayed()) await resume.click();
    const dialog = await $('dialog[open]');
    await dialog.waitForDisplayed({ timeout: UI_TIMEOUT });
    for (const name of ['FlatPack', 'DeepPack']) {
      const rows = await $$('dialog[open] tbody tr');
      let matching: WebdriverIO.Element | undefined;
      for (const row of rows) {
        if ((await row.getText()).includes(name)) {
          matching = row;
          break;
        }
      }
      if (!matching) throw new Error(`Import row for ${name} did not appear`);
      const sourceName = await matching.$('td:nth-child(2) span.font-semibold');
      await sourceName.waitForDisplayed({ timeout: UI_TIMEOUT });
      await sourceName.moveTo();
      const card = await $('body > aside');
      await card.waitForDisplayed({ timeout: UI_TIMEOUT });
      await (await card.$(`p=${NOTICE}`)).waitForDisplayed({ timeout: UI_TIMEOUT });
      expect(await card.$('h4').getText()).toContain(name);
      await (await $('dialog[open] h2')).moveTo();
      await card.waitForDisplayed({ reverse: true, timeout: UI_TIMEOUT });
    }
    const batch = await invokeInApp<ImportBatch>('get_import_batch', { batchId: previewBatchId });
    const deepItem = batch.items.find((item) => item.sourcePath.includes('DeepPack'));
    if (!deepItem) throw new Error('Deep source item is absent');
    const deep = await invokeInApp<ImportSourcePreview>('get_import_source_preview', {
      itemId: deepItem.id,
    });
    expect(deep.truncated).toBe(true);
  });
});
