import { browser, $, $$, expect } from '@wdio/globals';
import { BaseDirectory } from '@tauri-apps/api/path';
import { createServer, type Server } from 'node:http';
import { createHash } from 'node:crypto';
import { DatabaseSync } from 'node:sqlite';
import fs from 'node:fs/promises';
import path from 'node:path';
import type {
  AppSettings,
  BrowserBookmark,
  BrowserDownloadDto,
  BrowserHistoryEntry,
  BrowserSessionTab,
} from '../../../src/shared/api/tauri/bindings.gen.js';
import { createMockGame, scheduleMockGameRemoval, type MockGame } from '../support/fixtures.js';
import { gotoWorkspaceView, seedGamesAndOpenDashboard } from '../support/app.js';
import { waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';

const E2E_IDENTIFIER = 'com.reynalivan.emmm.e2e';
const APP_URL = 'http://tauri.localhost/';
const WAIT_TIMEOUT = 30_000;
const DATABASE_BUSY_TIMEOUT_MS = 2_500;
const FILTER_TIMESTAMP_KEY = 'adblock_last_success_at';
const ARTIFACT_ROOT = path.resolve('logs/client-smoke-20261007/browser');
const ARTIFACT_STAMP = new Date().toISOString().replace(/[^a-zA-Z0-9_-]/g, '-');
const ZIP_FIXTURE = path.resolve('tests/e2e/fixtures/sample-mod.zip');
const FIRST_TITLE = 'EMMM QA Page One';
const SECOND_TITLE = 'EMMM QA Page Two';
const ADDRESS_EDIT_PREFIX = 'http://1';

interface BrowserEventObservation {
  title?: string;
  loading?: boolean;
  activeTitle: string | null;
}

interface FilterFixtureSeed {
  databasePath: string;
  previousValue: string | null;
  seededValue: string;
}

function button(label: string): string {
  return `.//button[normalize-space(.)="${label}"]`;
}

async function click(selector: string): Promise<void> {
  const control = await $(selector);
  await control.waitForClickable({ timeout: WAIT_TIMEOUT });
  await control.click();
}

function ownedRelativePath(root: string, candidate: string): string {
  const relative = path.relative(path.toNamespacedPath(root), path.toNamespacedPath(candidate));
  if (!relative || path.isAbsolute(relative) || relative.startsWith('..')) {
    throw new Error(`Browser fixture path escaped its owned root: ${candidate}`);
  }
  return relative;
}

function safeSettings(settings: AppSettings, gameId?: string, inbox?: string): AppSettings {
  if (!settings.hotkeys) throw new Error('Native QA settings omitted hotkeys');
  return {
    ...settings,
    language: 'en',
    hotkeys: { ...settings.hotkeys, enabled: false },
    diagnostics: { ...settings.diagnostics, telemetry_enabled: false },
    games: settings.games.map((game) =>
      game.id === gameId && inbox ? { ...game, ready_to_move_path: inbox } : game,
    ),
  };
}

function settingsConflict(error: unknown): error is Error {
  return (
    error instanceof Error &&
    error.message.startsWith('[IPC] save_settings failed:') &&
    error.message.includes('Settings changed since this screen was loaded.')
  );
}

async function seedGame(game: MockGame): Promise<string> {
  try {
    return (await seedGamesAndOpenDashboard([{ game, name: 'QA Local Browser' }]))[0];
  } catch (error) {
    if (!settingsConflict(error)) throw error;
    console.log(
      `EMMM_E2E_SETUP_RETRY ${JSON.stringify({
        operation: 'browser_seed.save_settings',
        originalError: error.message,
        retry: 1,
      })}`,
    );
  }
  const current = await invokeInApp<AppSettings>('get_settings');
  await invokeInApp('save_settings', { settings: safeSettings(current) });
  const candidate = await invokeInApp<{ id: string }>('add_game_manual', {
    gameType: 'GIMI',
    path: game.root,
  });
  await invokeInApp('save_onboarding_games', {
    games: [{ ...candidate, name: 'QA Local Browser' }],
  });
  await invokeInApp('set_active_game', { gameId: candidate.id });
  await browser.url(APP_URL);
  await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: WAIT_TIMEOUT });
  await waitForWorkspaceCoreReady(candidate.id, WAIT_TIMEOUT);
  return candidate.id;
}

async function saveInboxSettings(gameId: string, inbox: string): Promise<void> {
  for (let attempt = 0; attempt < 2; attempt += 1) {
    const current = await invokeInApp<AppSettings>('get_settings');
    try {
      await invokeInApp('save_settings', { settings: safeSettings(current, gameId, inbox) });
      return;
    } catch (error) {
      if (attempt !== 0 || !settingsConflict(error)) throw error;
      console.log(
        `EMMM_E2E_SETUP_RETRY ${JSON.stringify({
          operation: 'browser_inbox.save_settings',
          originalError: error.message,
          retry: 1,
        })}`,
      );
    }
  }
}

async function seedLocalFilterTimestamp(gameId: string): Promise<FilterFixtureSeed> {
  const identifier = await invokeInApp<string>('plugin:app|identifier');
  if (identifier !== E2E_IDENTIFIER)
    throw new Error('Refusing browser seed outside E2E identifier');
  const appData = await invokeInApp<string>('plugin:path|resolve_directory', {
    directory: BaseDirectory.AppData,
  });
  const appDataRoot = await fs.realpath(appData);
  if (path.basename(appDataRoot) !== E2E_IDENTIFIER) {
    throw new Error(`Unexpected E2E app-data root: ${appDataRoot}`);
  }
  const requestedPath = path.join(appDataRoot, 'app.db');
  const databaseStat = await fs.lstat(requestedPath);
  if (!databaseStat.isFile() || databaseStat.isSymbolicLink()) {
    throw new Error('Private browser fixture DB must be an existing regular file');
  }
  const databasePath = await fs.realpath(requestedPath);
  if (path.dirname(databasePath) !== appDataRoot)
    throw new Error('Private fixture DB path escaped');
  const database = new DatabaseSync(databasePath, { allowExtension: false });
  try {
    database.exec(`PRAGMA busy_timeout = ${DATABASE_BUSY_TIMEOUT_MS}`);
    const columns = database.prepare('PRAGMA table_info(browser_settings)').all();
    if (
      columns.length !== 2 ||
      !columns.some(
        (column) => column.name === 'key' && column.type === 'TEXT' && column.pk === 1,
      ) ||
      !columns.some(
        (column) => column.name === 'value' && column.type === 'TEXT' && column.notnull === 1,
      ) ||
      !database
        .prepare('PRAGMA table_list')
        .all()
        .some((table) => table.name === 'browser_settings' && table.strict === 1)
    ) {
      throw new Error('Private E2E browser_settings schema did not match source contract');
    }
    const fixtureGame = database.prepare('SELECT id FROM games WHERE id = ?').get(gameId);
    if (fixtureGame?.id !== gameId)
      throw new Error('IPC fixture game absent from exact E2E database');
    const previous = database
      .prepare('SELECT value FROM browser_settings WHERE key = ?')
      .get(FILTER_TIMESTAMP_KEY);
    if (previous && typeof previous.value !== 'string')
      throw new Error('Invalid filter timestamp row');
    const previousValue = previous ? String(previous.value) : null;
    const seededValue = String(Math.floor(Date.now() / 1_000));
    database
      .prepare(
        'INSERT INTO browser_settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value',
      )
      .run(FILTER_TIMESTAMP_KEY, seededValue);
    const saved = database
      .prepare('SELECT value FROM browser_settings WHERE key = ?')
      .get(FILTER_TIMESTAMP_KEY);
    if (saved?.value !== seededValue)
      throw new Error('Private browser timestamp seed did not persist');
    const seed = { databasePath, previousValue, seededValue };
    await fs.mkdir(ARTIFACT_ROOT, { recursive: true });
    await fs.writeFile(
      path.join(ARTIFACT_ROOT, 'filter-fixture-seed.json'),
      JSON.stringify({ ...seed, identifier, certifiesFilterQuality: false }, null, 2),
    );
    console.log(
      `EMMM_QA_BROWSER_SETUP ${JSON.stringify({ ...seed, identifier, certifiesFilterQuality: false })}`,
    );
    return seed;
  } finally {
    database.close();
  }
}

async function restoreLocalFilterTimestamp(seed: FilterFixtureSeed): Promise<void> {
  const database = new DatabaseSync(seed.databasePath, { allowExtension: false });
  try {
    database.exec(`PRAGMA busy_timeout = ${DATABASE_BUSY_TIMEOUT_MS}`);
    const current = database
      .prepare('SELECT value FROM browser_settings WHERE key = ?')
      .get(FILTER_TIMESTAMP_KEY);
    if (current?.value !== seed.seededValue) {
      throw new Error('Private filter timestamp changed outside this fixture; refusing overwrite');
    }
    if (seed.previousValue === null) {
      database
        .prepare('DELETE FROM browser_settings WHERE key = ? AND value = ?')
        .run(FILTER_TIMESTAMP_KEY, seed.seededValue);
    } else {
      database
        .prepare('UPDATE browser_settings SET value = ? WHERE key = ? AND value = ?')
        .run(seed.previousValue, FILTER_TIMESTAMP_KEY, seed.seededValue);
    }
    const restored = database
      .prepare('SELECT value FROM browser_settings WHERE key = ?')
      .get(FILTER_TIMESTAMP_KEY);
    if ((restored?.value ?? null) !== seed.previousValue) {
      throw new Error('Private filter timestamp restoration did not match its captured row');
    }
    console.log(
      `EMMM_QA_BROWSER_SETUP_RESTORED ${JSON.stringify({ databasePath: seed.databasePath, restoredValue: seed.previousValue })}`,
    );
    await fs.writeFile(
      path.join(ARTIFACT_ROOT, 'filter-fixture-restoration.json'),
      JSON.stringify(
        { databasePath: seed.databasePath, restoredValue: seed.previousValue },
        null,
        2,
      ),
    );
  } finally {
    database.close();
  }
}

async function addressSubmit(url: string): Promise<void> {
  const draft = await $('#browser-url-input');
  await draft.waitForClickable({ timeout: WAIT_TIMEOUT });
  await draft.click();
  const input = await $('input#browser-url-input');
  await input.waitForDisplayed({ timeout: WAIT_TIMEOUT });
  await browser.keys(['Control', 'a']);
  await browser.keys(['Backspace']);
  await browser.keys(Array.from(url));
  await expect(input).toHaveValue(url);
  console.log(
    `EMMM_QA_BROWSER_ADDRESS ${JSON.stringify(
      await browser.execute(() => {
        const input = document.getElementById('browser-url-input');
        return {
          value: input instanceof HTMLInputElement ? input.value : input?.getAttribute('title'),
          tag: input?.tagName,
          focused: input === document.activeElement,
          activeElement: document.activeElement?.id,
        };
      }),
    )}`,
  );
  await browser.keys(['Enter']);
  const consent = await $('dialog[open]');
  await consent.waitForDisplayed({ timeout: WAIT_TIMEOUT });
  await expect(consent).toHaveText(/Confirm action/);
  await expect(consent).toHaveText(/not using HTTPS/);
  await consent.$(button('Confirm')).waitForClickable({ timeout: WAIT_TIMEOUT });
  await consent.$(button('Confirm')).click();
  await consent.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
}

async function toolbarMenu(): Promise<WebdriverIO.Element> {
  await click('button[aria-label="Discover browser menu"]');
  const menu = await $('[data-testid="browser-toolbar-menu"]');
  await menu.waitForDisplayed({ timeout: WAIT_TIMEOUT });
  return menu.getElement();
}

async function nativeTabCount(): Promise<number> {
  return (await $$('[role="tablist"][aria-label="Discover"] button[role="tab"]')).length;
}

describe('Native localhost Discover and Downloads — owned QA profile', () => {
  let game: MockGame;
  let gameId: string;
  let inbox: string;
  let server: Server | undefined;
  let baseUrl: string;
  let filterSeed: FilterFixtureSeed | undefined;
  let zip: Buffer;
  const requestCounts = new Map<string, number>();

  const firstUrl = () => `${baseUrl}/one`;
  const secondUrl = () => `${baseUrl}/two`;
  const zipUrl = () => `${baseUrl}/QAArchive.zip`;

  async function waitPage(title: string, url: string): Promise<void> {
    await expect(
      await $('[role="tablist"][aria-label="Discover"] button[role="tab"][aria-selected="true"]'),
    ).toHaveText(title);
    await expect(await $('#browser-url-input')).toHaveAttribute('title', url);
    await browser.waitUntil(
      async () => {
        const tabs = await invokeInApp<BrowserSessionTab[]>('browser_get_session_tabs', { gameId });
        return tabs.some((tab) => tab.active && tab.url === url && tab.title === title);
      },
      { timeout: WAIT_TIMEOUT, timeoutMsg: `Native session did not save ${title} at owned URL` },
    );
  }

  async function downloads(): Promise<BrowserDownloadDto[]> {
    return invokeInApp<BrowserDownloadDto[]>('browser_list_downloads', { gameId });
  }

  async function downloadRow(filename: string): Promise<WebdriverIO.Element> {
    const row = await $(`.//tr[.//p[@title="${filename}"]]`);
    await row.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    return row.getElement();
  }

  async function downloadAction(filename: string, action: string): Promise<void> {
    const row = await downloadRow(filename);
    await row
      .$('button[aria-label="Download actions"]')
      .waitForClickable({ timeout: WAIT_TIMEOUT });
    await row.$('button[aria-label="Download actions"]').click();
    const menu = await $('[role="menu"][aria-label="Download actions"]');
    await menu.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await menu.$(button(action)).click();
  }

  async function stopServer(): Promise<void> {
    if (!server) return;
    const owned = server;
    server = undefined;
    owned.closeAllConnections();
    await new Promise<void>((resolve, reject) =>
      owned.close((error) => (error ? reject(error) : resolve())),
    );
  }

  async function captureFailure(scenario: string, error: unknown, phase: string): Promise<void> {
    await fs.mkdir(ARTIFACT_ROOT, { recursive: true });
    const name = `${ARTIFACT_STAMP}-${phase}-${scenario.replace(/[^a-zA-Z0-9_-]/g, '_')}`;
    const screenshot = path.join(ARTIFACT_ROOT, `${name}.png`);
    const dom = path.join(ARTIFACT_ROOT, `${name}.html`);
    const metadata = path.join(ARTIFACT_ROOT, `${name}.json`);
    await browser.saveScreenshot(screenshot);
    await fs.writeFile(dom, await browser.getPageSource(), 'utf8');
    const observation = await browser.execute(() => {
      const input = document.getElementById('browser-url-input');
      return {
        documentUrl: location.href,
        urlInput: {
          tag: input?.tagName,
          value: input instanceof HTMLInputElement ? input.value : input?.getAttribute('title'),
          focused: input === document.activeElement,
        },
        activeElement: document.activeElement?.id,
        dialogs: Array.from(document.querySelectorAll('dialog, [role="dialog"]')).map((dialog) => ({
          tag: dialog.tagName,
          open: dialog instanceof HTMLDialogElement ? dialog.open : null,
          label: dialog.getAttribute('aria-labelledby'),
          text: dialog.textContent?.slice(0, 250),
          bounds: dialog.getBoundingClientRect().toJSON(),
        })),
        panels: Array.from(document.querySelectorAll('aside')).map((panel) => ({
          label: panel.getAttribute('aria-label'),
          bounds: panel.getBoundingClientRect().toJSON(),
        })),
      };
    });
    const tabs = await invokeInApp<BrowserSessionTab[]>('browser_get_session_tabs', { gameId });
    const nativeEvents = await browser.execute(
      () =>
        (window as Window & { __qaBrowserEvents?: BrowserEventObservation[] }).__qaBrowserEvents ??
        [],
    );
    const history = await invokeInApp<BrowserHistoryEntry[]>('browser_list_history', { limit: 20 });
    const record = {
      scenario,
      phase,
      error: error instanceof Error ? error.message : String(error),
      screenshot,
      dom,
      metadata,
      expectedUrl: firstUrl(),
      serverRequestCounts: Object.fromEntries(requestCounts),
      observation,
      tabs,
      nativeEvents,
      history,
    };
    await fs.writeFile(metadata, JSON.stringify(record, null, 2));
    console.log(`EMMM_E2E_FAILURE ${JSON.stringify(record)}`);
  }

  before(async () => {
    game = await createMockGame('QALocalBrowser');
    scheduleMockGameRemoval(game);
    gameId = await seedGame(game);
    inbox = path.join(game.root, 'QAInbox');
    await fs.mkdir(inbox);
    await saveInboxSettings(gameId, inbox);
    zip = await fs.readFile(ZIP_FIXTURE);
    filterSeed = await seedLocalFilterTimestamp(gameId);
    server = createServer((request, response) => {
      const route = (request.url ?? '').split('?')[0];
      requestCounts.set(route, (requestCounts.get(route) ?? 0) + 1);
      if (route === '/QAArchive.zip') {
        response.writeHead(200, {
          'Content-Type': 'application/zip',
          'Content-Disposition': 'attachment; filename="QAArchive.zip"',
          'Content-Length': zip.length,
          'Cache-Control': 'no-store',
          'Accept-Ranges': 'none',
        });
        response.end(request.method === 'HEAD' ? undefined : zip);
        return;
      }
      if (route !== '/one' && route !== '/two') {
        response.writeHead(204);
        response.end();
        return;
      }
      const title = route === '/one' ? FIRST_TITLE : SECOND_TITLE;
      const page = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>${title}</title><link rel="icon" href="data:,"><meta name="viewport" content="width=device-width,initial-scale=1"></head><body><h1>${title}</h1><p>QA find marker</p><a href="/one">QA first page</a><a href="/two">QA second page</a><a href="/QAArchive.zip">QA archive</a></body></html>`;
      response.writeHead(200, {
        'Content-Type': 'text/html; charset=utf-8',
        'Content-Length': Buffer.byteLength(page),
        'Cache-Control': 'no-store',
        'Content-Security-Policy':
          "default-src 'none'; img-src data:; style-src 'none'; connect-src 'none'",
      });
      response.end(request.method === 'HEAD' ? undefined : page);
    });
    server.requestTimeout = 5_000;
    server.headersTimeout = 5_000;
    await new Promise<void>((resolve, reject) => {
      if (!server) throw new Error('Owned server was not constructed');
      server.once('error', reject);
      server.listen(0, '127.0.0.1', () => {
        server?.off('error', reject);
        resolve();
      });
    });
    const address = server.address();
    if (!address || typeof address === 'string' || address.address !== '127.0.0.1') {
      throw new Error('QA server did not bind its owned loopback port');
    }
    baseUrl = `http://127.0.0.1:${address.port}`;
    console.log(
      `EMMM_QA_BROWSER_FIXTURE ${JSON.stringify({ baseUrl, gameId, fixtureRoot: game.fixtureRoot, inbox })}`,
    );
  });

  beforeEach(async function () {
    try {
      await browser.url(APP_URL);
      await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: WAIT_TIMEOUT });
      await gotoWorkspaceView('browser');
      await $('#browser-url-input').waitForClickable({ timeout: WAIT_TIMEOUT });
      if (this.currentTest?.title.startsWith('keeps a blank address textbox')) return;
      const observerError = await browser.executeAsync((done: (result: string | null) => void) => {
        const target = window as unknown as Window & {
          __qaBrowserEvents?: BrowserEventObservation[];
          __TAURI__: {
            event: {
              listen: (
                name: string,
                handler: (event: { payload: { title?: string; loading?: boolean } }) => void,
              ) => Promise<() => void>;
            };
          };
        };
        target.__qaBrowserEvents = [];
        const record = (event: { payload: { title?: string; loading?: boolean } }) => {
          target.__qaBrowserEvents?.push({
            title: event.payload.title,
            loading: event.payload.loading,
            activeTitle:
              document.querySelector(
                '[role="tablist"][aria-label="Discover"] [aria-selected="true"]',
              )?.textContent ?? null,
          });
        };
        Promise.all([
          target.__TAURI__.event.listen('browser:url-changed', record),
          target.__TAURI__.event.listen('browser:loading-changed', record),
        ]).then(
          () => done(null),
          (error: unknown) => done(String(error)),
        );
      });
      if (observerError) throw new Error(`Native event observer setup failed: ${observerError}`);
      await addressSubmit(firstUrl());
      await waitPage(FIRST_TITLE, firstUrl());
    } catch (error) {
      await captureFailure(this.currentTest?.title ?? 'browser_setup', error, 'beforeEach');
      throw error;
    }
  });

  afterEach(async function () {
    if (this.currentTest?.state !== 'failed') return;
    await captureFailure(this.currentTest.title, this.currentTest.err, 'test');
  });

  after(async () => {
    try {
      await stopServer();
    } finally {
      if (filterSeed) await restoreLocalFilterTimestamp(filterSeed);
    }
  });

  it('keeps a blank address textbox focused while a valid localhost prefix is entered', async () => {
    await click('button[aria-label="New Tab"]');
    const input = await $('input#browser-url-input');
    await input.waitForClickable({ timeout: WAIT_TIMEOUT });
    await input.click();
    await browser.keys(Array.from(ADDRESS_EDIT_PREFIX));
    await expect(await $('input#browser-url-input')).toExist();
    await expect(await $('input#browser-url-input')).toBeFocused();
    await expect(await $('input#browser-url-input')).toHaveValue(ADDRESS_EDIT_PREFIX);
  });

  it('navigates owned native pages with Back Forward Reload Find Zoom and child-tab recovery', async () => {
    await addressSubmit(secondUrl());
    await waitPage(SECOND_TITLE, secondUrl());
    await click('button[title="Back"]');
    await waitPage(FIRST_TITLE, firstUrl());
    await click('button[title="Forward"]');
    await waitPage(SECOND_TITLE, secondUrl());
    const beforeReload = requestCounts.get('/two') ?? 0;
    await click('button[title="Refresh"]');
    await browser.waitUntil(() => (requestCounts.get('/two') ?? 0) > beforeReload, {
      timeout: WAIT_TIMEOUT,
      timeoutMsg: 'Rendered Reload did not reach owned server',
    });
    await waitPage(SECOND_TITLE, secondUrl());
    let menu = await toolbarMenu();
    await menu.$('.//button[starts-with(normalize-space(.),"Find in page")]').click();
    const find = await $('input[placeholder="Find in page"]');
    await find.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await find.setValue('QA find marker');
    await click(button('Find'));
    await find.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    menu = await toolbarMenu();
    await menu.$('button[aria-label="Zoom in"]').click();
    await expect(await menu.$(button('110%'))).toBeDisplayed();
    await menu.$('button[aria-label="Zoom out"]').click();
    await expect(await menu.$(button('100%'))).toBeDisabled();
    await click('button[aria-label="Discover browser menu"]');
    const tabCount = await nativeTabCount();
    await $('[role="tablist"][aria-label="Discover"] button[aria-selected="true"]').click({
      button: 'right',
    });
    await click(button('Duplicate tab'));
    await browser.waitUntil(async () => (await nativeTabCount()) === tabCount + 1, {
      timeout: WAIT_TIMEOUT,
    });
    await waitPage(SECOND_TITLE, secondUrl());
    await $('[role="tablist"][aria-label="Discover"] button[aria-selected="true"]').click({
      button: 'right',
    });
    await click(button('Close tab'));
    await browser.waitUntil(async () => (await nativeTabCount()) === tabCount, {
      timeout: WAIT_TIMEOUT,
    });
    await $('[role="tablist"][aria-label="Discover"] button[aria-selected="true"]').click({
      button: 'right',
    });
    await click(button('Reopen last closed tab'));
    await waitPage(SECOND_TITLE, secondUrl());
  });

  it('adds edits validates navigates and removes only its local bookmark and clears private history through confirmation', async () => {
    await click('button[aria-label="Add bookmark"]');
    await expect(await $('button[aria-label="Remove bookmark"]')).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    const saved = (await invokeInApp<BrowserBookmark[]>('browser_list_bookmarks')).find(
      (item) => item.url === firstUrl(),
    );
    if (!saved) throw new Error('Rendered bookmark action did not persist owned URL');
    let menu = await toolbarMenu();
    await menu.$(button('Bookmarks')).click();
    let library = await $('aside[aria-label="Bookmarks & history"]');
    await library.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    const ownRow = await library.$(`.//*[@role="listitem" and .//button[@title="${firstUrl()}"]]`);
    await ownRow.$('button[aria-label="Edit bookmark"]').click();
    const editor = await $('dialog[aria-labelledby="bookmark-editor-title"][open]');
    await editor.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    const name = await editor.$('.//label[.//span[normalize-space(.)="Name"]]//input');
    const address = await editor.$('.//label[.//span[normalize-space(.)="Address"]]//input');
    await address.setValue('');
    await expect(await editor.$(button('Save changes'))).toBeDisabled();
    await address.setValue('invalid address');
    await editor.$(button('Save changes')).click();
    expect(await address.getProperty('validationMessage')).not.toBe('');
    await expect(editor).toBeDisplayed();
    await address.setValue('file:///QA-invalid-scheme');
    await editor.$(button('Save changes')).click();
    await expect(await $('*=Discover browser action failed. Please try again.')).toBeDisplayed();
    await expect(editor).toBeDisplayed();
    expect(
      (await invokeInApp<BrowserBookmark[]>('browser_list_bookmarks')).find(
        (item) => item.id === saved.id,
      )?.url,
    ).toBe(firstUrl());
    await address.setValue(secondUrl());
    await name.setValue('QA Edited Bookmark');
    await editor.$(button('Save changes')).click();
    await editor.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    const changed = (await invokeInApp<BrowserBookmark[]>('browser_list_bookmarks')).find(
      (item) => item.id === saved.id,
    );
    expect(changed?.title).toBe('QA Edited Bookmark');
    expect(changed?.url).toBe(secondUrl());
    await library.$(`button[title="${secondUrl()}"]`).click();
    const consent = await $('dialog[open]');
    await consent.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await consent.$(button('Confirm')).click();
    await consent.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    await waitPage(SECOND_TITLE, secondUrl());
    await expect(await $('aside[aria-label="Bookmarks & history"]')).not.toExist();
    menu = await toolbarMenu();
    await menu.$(button('Bookmarks')).click();
    library = await $('aside[aria-label="Bookmarks & history"]');
    await library.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await library
      .$(`.//*[@role="listitem" and .//button[@title="${secondUrl()}"]]`)
      .$('button[aria-label="Remove bookmark"]')
      .click();
    await browser.waitUntil(
      async () =>
        !(await invokeInApp<BrowserBookmark[]>('browser_list_bookmarks')).some(
          (item) => item.id === saved.id,
        ),
      { timeout: WAIT_TIMEOUT },
    );
    await library.$('.//button[@role="tab" and normalize-space(.)="History"]').click();
    await library.$(`.//button[.//span[normalize-space(.)="${FIRST_TITLE}"]]`).click();
    const historyConsent = await $('dialog[open]');
    await historyConsent.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await historyConsent.$(button('Confirm')).click();
    await historyConsent.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    await waitPage(FIRST_TITLE, firstUrl());
    await expect(await $('aside[aria-label="Bookmarks & history"]')).not.toExist();
    menu = await toolbarMenu();
    await menu.$(button('History')).click();
    library = await $('aside[aria-label="Bookmarks & history"]');
    await library.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    const before = await invokeInApp<BrowserHistoryEntry[]>('browser_list_history', { limit: 100 });
    expect(before.some((entry) => entry.url === firstUrl())).toBe(true);
    await library.$(button('Clear history')).click();
    let confirmation = await $('dialog[open]');
    await confirmation.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await confirmation.$(button('Cancel')).click();
    expect(
      await invokeInApp<BrowserHistoryEntry[]>('browser_list_history', { limit: 100 }),
    ).toEqual(before);
    await library.$(button('Clear history')).click();
    confirmation = await $('dialog[open]');
    await confirmation.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await confirmation.$(button('Confirm')).click();
    await expect(await library.$('p=No browsing history yet.')).toBeDisplayed();
    expect(
      await invokeInApp<BrowserHistoryEntry[]>('browser_list_history', { limit: 100 }),
    ).toHaveLength(0);
    await click('button[aria-label="Close bookmarks and history"]');
  });

  it('toggles ad block and confirms private-profile cache and site-data actions without certifying filter quality', async () => {
    const original = await invokeInApp<boolean>('browser_get_adblock_enabled');
    let menu = await toolbarMenu();
    await menu.$('button[role="switch"][aria-label="Ad block"]').click();
    expect(await invokeInApp<boolean>('browser_get_adblock_enabled')).toBe(!original);
    menu = await toolbarMenu();
    await menu.$('button[role="switch"][aria-label="Ad block"]').click();
    expect(await invokeInApp<boolean>('browser_get_adblock_enabled')).toBe(original);
    for (const [label, feedback] of [
      ['Clear cache', 'Discover cache cleared.'],
      ['Clear cookies & site data', 'Cookies and site data cleared for Discover.'],
    ]) {
      menu = await toolbarMenu();
      await menu.$(button(label)).click();
      let confirmation = await $('dialog[open]');
      await confirmation.waitForDisplayed({ timeout: WAIT_TIMEOUT });
      await confirmation.$(button('Cancel')).click();
      await confirmation.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
      await waitPage(FIRST_TITLE, firstUrl());
      menu = await toolbarMenu();
      await menu.$(button(label)).click();
      confirmation = await $('dialog[open]');
      await confirmation.waitForDisplayed({ timeout: WAIT_TIMEOUT });
      await confirmation.$(button('Confirm')).click();
      await confirmation.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
      await expect(await $(`*=${feedback}`)).toBeDisplayed();
      await waitPage(FIRST_TITLE, firstUrl());
    }
    const settings = await invokeInApp<AppSettings>('get_settings');
    expect(settings.active_game_id).toBe(gameId);
    expect(settings.hotkeys?.enabled).toBe(false);
    expect(settings.diagnostics?.telemetry_enabled).toBe(false);
  });

  it('downloads the owned ZIP through metadata consent, validates renames, cancels deletion and removes its record while retaining bytes', async () => {
    await addressSubmit(zipUrl());
    let consent = await $('dialog[aria-labelledby="download-confirmation-title"]');
    await consent.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await expect(consent).toHaveText(/QAArchive.zip/);
    await expect(consent).toHaveText(/127\.0\.0\.1/);
    await expect(consent).toHaveText(/QAInbox/);
    await consent.$(button('Cancel')).click();
    await consent.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    expect(
      (await downloads()).some(
        (item) => item.source_url === zipUrl() && item.status === 'finished',
      ),
    ).toBe(false);
    expect(await fs.readdir(inbox)).not.toContain('QAArchive.zip');
    await addressSubmit(zipUrl());
    consent = await $('dialog[aria-labelledby="download-confirmation-title"]');
    await consent.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await consent.$(button('Download')).click();
    await consent.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    let downloaded: BrowserDownloadDto | undefined;
    await browser.waitUntil(
      async () => {
        downloaded = (await downloads()).find(
          (item) => item.source_url === zipUrl() && item.status === 'finished',
        );
        return Boolean(downloaded?.file_path);
      },
      { timeout: WAIT_TIMEOUT, timeoutMsg: 'Native owned ZIP download did not finish' },
    );
    if (!downloaded?.file_path) throw new Error('Finished download omitted owned file path');
    ownedRelativePath(game.fixtureRoot, downloaded.file_path);
    expect(
      createHash('sha256')
        .update(await fs.readFile(downloaded.file_path))
        .digest('hex'),
    ).toBe(createHash('sha256').update(zip).digest('hex'));
    const id = downloaded.id;
    const originalPath = downloaded.file_path;
    await gotoWorkspaceView('downloads');
    await expect(await (await downloadRow('QAArchive.zip')).$('span=Ready')).toBeDisplayed();
    await fs.writeFile(path.join(inbox, 'QACollision.zip'), 'QA collision sentinel');
    await downloadAction('QAArchive.zip', 'Rename file');
    let rename = await $('dialog[open][aria-labelledby^="download-rename-"]');
    await rename.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    let filename = await rename.$('.//label[.//span[normalize-space(.)="Filename"]]//input');
    await filename.setValue('');
    await expect(await rename.$(button('Save name'))).toBeDisabled();
    await rename.$(button('Cancel')).click();
    expect((await downloads()).find((item) => item.id === id)?.filename).toBe('QAArchive.zip');
    await downloadAction('QAArchive.zip', 'Rename file');
    rename = await $('dialog[open][aria-labelledby^="download-rename-"]');
    filename = await rename.$('.//label[.//span[normalize-space(.)="Filename"]]//input');
    for (const invalid of ['CON.zip', 'QACollision.zip']) {
      await filename.setValue(invalid);
      await rename.$(button('Save name')).click();
      await expect(await $('*=Could not rename this file. Please try again.')).toBeDisplayed();
      await expect(rename).toBeDisplayed();
      expect((await downloads()).find((item) => item.id === id)?.filename).toBe('QAArchive.zip');
      expect(await fs.readFile(originalPath)).toEqual(zip);
    }
    expect(await fs.readFile(path.join(inbox, 'QACollision.zip'), 'utf8')).toBe(
      'QA collision sentinel',
    );
    await filename.setValue('QARenamed.zip');
    await rename.$(button('Save name')).click();
    await rename.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    await downloadRow('QARenamed.zip');
    const renamed = (await downloads()).find((item) => item.id === id);
    expect(renamed?.filename).toBe('QARenamed.zip');
    expect(renamed?.source_url).toBe(zipUrl());
    if (!renamed?.file_path) throw new Error('Renamed native record omitted local file');
    ownedRelativePath(game.fixtureRoot, renamed.file_path);
    expect(await fs.readFile(renamed.file_path)).toEqual(zip);
    expect(await fs.readdir(inbox)).not.toContain(path.basename(originalPath));
    await downloadAction('QARenamed.zip', 'Delete file');
    const deletion = await $('dialog[open]');
    await deletion.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await expect(deletion).toHaveText(/Delete downloaded file\?/);
    await deletion.$(button('Cancel')).click();
    await deletion.waitForDisplayed({ reverse: true, timeout: WAIT_TIMEOUT });
    expect(await fs.readFile(renamed.file_path)).toEqual(zip);
    expect((await downloads()).find((item) => item.id === id)?.filename).toBe('QARenamed.zip');
    await downloadAction('QARenamed.zip', 'Remove from list');
    await browser.waitUntil(async () => !(await downloads()).some((item) => item.id === id), {
      timeout: WAIT_TIMEOUT,
    });
    await expect(await $('.//tr[.//p[@title="QARenamed.zip"]]')).not.toExist();
    expect(await fs.readFile(renamed.file_path)).toEqual(zip);
    console.log(
      `EMMM_QA_BROWSER_DOWNLOAD ${JSON.stringify({ id, filePath: renamed.file_path, fixtureRoot: game.fixtureRoot, recordRemoved: true, bytesPreserved: true })}`,
    );
  });

  it('shows a native owned-host offline error and its Try again action after its server stops', async () => {
    await stopServer();
    await addressSubmit(`${baseUrl}/offline`);
    const error = await $('h2=This page could not load');
    await error.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await expect(await $(button('Open externally'))).toBeDisplayed();
    await click(button('Try again'));
    await error.waitForDisplayed({ timeout: WAIT_TIMEOUT });
    await expect(await $('#browser-url-input')).toHaveAttribute('title', `${baseUrl}/offline`);
  });

  it.skip(
    'BLOCKED: tiny nonresumable ZIP cannot expose bounded Pause/Resume; native image/Base64 page context menu and OS-launch controls require separate native interaction coverage',
  );
});
