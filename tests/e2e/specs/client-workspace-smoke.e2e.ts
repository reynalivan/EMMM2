import { $, $$, browser, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
import type { ChainablePromiseElement } from 'webdriverio';
import {
  addMockMod,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { gotoWorkspaceView, seedGamesAndOpenDashboard } from '../support/app.js';
import { getObjects, reconcile, waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';
import type {
  CollectionPreview,
  CollectionRuntimeSnapshot,
  CollectionSummary,
  BulkResult,
} from '../../../src/shared/api/tauri/bindings.gen.js';

const UI_TIMEOUT = 15_000;
const CASE_TIMEOUT = 90_000;
const ALPHA = 'QA Alpha';
const BETA = 'QA Beta';
const GAMMA = 'QA Gamma';
const MAIN_GAME = 'EMMM QA workspace';
const OTHER_GAME = 'EMMM QA second game';
const ARTIFACT_DIRECTORY = path.resolve('logs/client-smoke-20261007/workspace');
const PNG_BASE64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==';
const PREVIEW_INI =
  '[TextureOverrideQAPreview]\nhash = 00000f01\n\n[Constants]\nglobal $qa = 0\n\n[KeyQA]\nkey = VK_F6\ntype = cycle\n$qa = 0,1\n';

function xpathText(value: string): string {
  if (!value.includes("'")) return `'${value}'`;
  if (!value.includes('"')) return `"${value}"`;
  return `concat(${value
    .split("'")
    .map((part) => `'${part}'`)
    .join(', "\'", ')})`;
}

async function exists(target: string): Promise<boolean> {
  try {
    await fs.access(target);
    return true;
  } catch (error: unknown) {
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return false;
    throw error;
  }
}

async function click(selector: string): Promise<void> {
  const control = await $(selector);
  await control.waitForClickable({ timeout: UI_TIMEOUT });
  await control.click();
}

async function replaceText(control: ChainablePromiseElement, value: string): Promise<void> {
  await control.waitForClickable({ timeout: UI_TIMEOUT });
  await control.click();
  await browser.keys(['Control', 'a']);
  await browser.keys(['Backspace']);
  if (value.length > 0) await browser.keys(Array.from(value));
}

async function visibleElement(selector: string): Promise<WebdriverIO.Element> {
  await browser.waitUntil(
    async () => {
      for (const candidate of await $$(selector)) {
        if (await candidate.isDisplayed()) return true;
      }
      return false;
    },
    { timeout: UI_TIMEOUT, timeoutMsg: `No displayed workspace control matches ${selector}` },
  );
  const candidates = await $$(selector);
  for (const candidate of candidates) {
    if (await candidate.isDisplayed()) return candidate;
  }
  throw new Error(`No displayed workspace control matches ${selector}`);
}

async function card(name: string): Promise<ChainablePromiseElement> {
  const item = await $(
    `//*[@data-testid="folder-grid"]//*[@role="gridcell"][.//h3[@title=${xpathText(name)}]]`,
  );
  await item.waitForDisplayed({ timeout: UI_TIMEOUT });
  return item;
}

async function openPreview(name: string): Promise<void> {
  await (await card(name)).click();
  const title = await $('[data-testid="workspace-right"] input[aria-label="Rename mod"]');
  await title.waitForDisplayed({ timeout: UI_TIMEOUT });
  await browser.waitUntil(async () => (await title.getValue()) === name, {
    timeout: UI_TIMEOUT,
    timeoutMsg: `Preview did not select ${name}`,
  });
}

async function switchCard(name: string): Promise<void> {
  const control = await (await card(name)).$('input[type="checkbox"][aria-label]');
  await control.waitForClickable({ timeout: UI_TIMEOUT });
  await control.click();
}

async function selectCard(name: string): Promise<void> {
  const item = await card(name);
  await item.moveTo();
  const selection = await item.$('input[type="checkbox"]:not([aria-label])');
  await selection.waitForClickable({ timeout: UI_TIMEOUT });
  await selection.click();
  await expect(selection).toBeChecked();
}

async function collectionRow(name: string): Promise<ChainablePromiseElement> {
  const row = await $(`//tr[.//span[normalize-space(.)=${xpathText(name)}]]`);
  await row.waitForDisplayed({ timeout: UI_TIMEOUT });
  return row;
}

async function dialogButton(text: string): Promise<void> {
  await click(`//dialog[@open]//button[normalize-space(.)=${xpathText(text)}]`);
}

async function openRename(name: string): Promise<ChainablePromiseElement> {
  await openPreview(name);
  await click('button[title="More Actions"]');
  await click('//ul[@role="menu"]//button[normalize-space(.)="Rename"]');
  const input = await (await card(name)).$('h3 input');
  await input.waitForDisplayed({ timeout: UI_TIMEOUT });
  return input;
}

function normalizePath(value: string): string {
  return value.replaceAll('\\', '/').toLowerCase();
}

describe('Native client workspace controls on owned QA fixtures', function () {
  this.timeout(CASE_TIMEOUT);

  let game: MockGame;
  let secondGame: MockGame;
  let gameId = '';
  let secondGameId = '';
  let previewPath = '';
  let objectIds: Record<string, string> = {};
  let beforeEachFailure: Error | null = null;

  async function selectGame(name: string, id: string): Promise<void> {
    const picker = await visibleElement('button[aria-label="Select Game"]');
    await picker.waitForClickable({ timeout: UI_TIMEOUT });
    await picker.click();
    const option = await (await picker.$('..')).$(`button=${name}`);
    await option.waitForClickable({ timeout: UI_TIMEOUT });
    await option.click();
    await waitForWorkspaceCoreReady(id, CASE_TIMEOUT);
    await browser.waitUntil(async () => (await picker.getText()).includes(name), {
      timeout: UI_TIMEOUT,
      timeoutMsg: `Game picker did not settle on ${name}`,
    });
  }

  async function openObject(name: string): Promise<void> {
    const id = objectIds[name];
    if (!id) throw new Error(`No fixture object id for ${name}`);
    await click(`[data-object-id="${id}"]`);
    await $('[data-testid="folder-grid"]').waitForDisplayed({ timeout: UI_TIMEOUT });
  }

  async function assertPrefix(object: string, name: string, enabled: boolean): Promise<void> {
    const directory = path.join(game.modsPath, object);
    const expectedName = enabled ? name : `DISABLED ${name}`;
    const absentName = enabled ? `DISABLED ${name}` : name;
    await browser.waitUntil(
      async () => {
        const entries = await fs.readdir(directory);
        return entries.includes(expectedName) && !entries.includes(absentName);
      },
      {
        timeout: UI_TIMEOUT,
        interval: 50,
        timeoutMsg: `Disk did not settle to ${path.join(directory, expectedName)}`,
      },
    );
    const control = await (await card(name)).$('input[type="checkbox"][aria-label]');
    if (enabled) await expect(control).toBeChecked();
    else await expect(control).not.toBeChecked();
  }

  async function runtime(): Promise<CollectionRuntimeSnapshot> {
    return invokeInApp<CollectionRuntimeSnapshot>('get_collection_runtime_state', { gameId });
  }

  async function storedPreview(id: string): Promise<CollectionPreview> {
    return invokeInApp<CollectionPreview>('get_collection_preview', {
      collectionId: id,
      gameId,
    });
  }

  async function listCollections(): Promise<CollectionSummary[]> {
    return invokeInApp<CollectionSummary[]>('list_collections', { gameId });
  }

  async function seedCollection(label: string): Promise<CollectionSummary> {
    return invokeInApp<CollectionSummary>('create_collection', {
      gameId,
      name: `${label} ${randomUUID()}`,
      saveMode: 'save_current_state',
    });
  }

  before(async function () {
    this.timeout(CASE_TIMEOUT);
    game = await createMockGame('QA_workspace');
    secondGame = await createMockGame('QA_workspace_second');
    for (const fixture of [game, secondGame]) {
      await fs.writeFile(
        path.join(fixture.root, 'd3dx.ini'),
        '[Include]\ninclude_recursive = Mods\n',
      );
    }
    // Initial fixture discovery uses the same pre-onboarding disk layout as native stability specs.
    const fixtures = [
      [ALPHA, 'Rapid'],
      [ALPHA, 'Bulk A'],
      [ALPHA, 'Bulk B'],
      [ALPHA, 'Preview'],
      [BETA, 'Shared'],
      [BETA, 'Rename Source'],
      [path.join(GAMMA, 'DISABLED Parent'), 'Shared'],
      [path.join(GAMMA, 'DISABLED Parent'), 'DISABLED Own Sentinel'],
    ] as const;
    for (const [index, [object, name]] of fixtures.entries()) {
      const mod = await addMockMod(game, object, name);
      await fs.writeFile(
        path.join(mod, 'mod.ini'),
        `[TextureOverrideQA${index}]\nhash = ${index.toString(16).padStart(8, '0')}\n`,
      );
      if (name === 'Preview') previewPath = mod;
      await fs.writeFile(path.join(mod, 'qa-sentinel.txt'), `${object}/${name}`);
    }
    await fs.writeFile(path.join(previewPath, 'mod.ini'), PREVIEW_INI);
    await fs.writeFile(
      path.join(previewPath, 'info.json'),
      JSON.stringify({ author: 'QA Original' }),
    );
    const second = await addMockMod(secondGame, 'Second Game Object', 'Second Game Only');
    await fs.writeFile(
      path.join(second, 'mod.ini'),
      '[TextureOverrideQASecond]\nhash = 12345678\n',
    );
    for (let index = 0; index < 2; index += 1) {
      const filename = index === 0 ? 'preview_custom.png' : 'preview_custom_1.png';
      await fs.writeFile(path.join(previewPath, filename), Buffer.from(PNG_BASE64, 'base64'));
    }
    [gameId, secondGameId] = await seedGamesAndOpenDashboard(
      [
        { game, name: MAIN_GAME },
        { game: secondGame, name: OTHER_GAME },
      ],
      CASE_TIMEOUT,
    );
    await browser.waitUntil(
      async () => {
        const objects = await getObjects(gameId);
        const expected = [ALPHA, BETA, GAMMA];
        if (objects.length !== expected.length) return false;
        const identified: Record<string, string> = {};
        for (const name of expected) {
          const object = objects.find((entry) => entry.name === name);
          if (!object) return false;
          identified[name] = object.id;
        }
        objectIds = identified;
        return true;
      },
      {
        timeout: CASE_TIMEOUT,
        interval: 100,
        timeoutMsg: 'Pre-onboarding fixture objects did not reach the indexed workspace read model',
      },
    );
    console.info(`WORKSPACE_QA_FIXTURE ${game.fixtureRoot}`);
  });

  async function arrangeWorkspace(): Promise<void> {
    await browser.url('http://tauri.localhost/');
    await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: UI_TIMEOUT });
    await gotoWorkspaceView('mods');
    const picker = await visibleElement('button[aria-label="Select Game"]');
    await picker.waitForDisplayed({ timeout: UI_TIMEOUT });
    if (!(await picker.getText()).includes(MAIN_GAME)) await selectGame(MAIN_GAME, gameId);
    const entries = await fs.readdir(path.join(game.modsPath, ALPHA));
    const restore = ['Rapid', 'Bulk A', 'Bulk B']
      .filter((name) => entries.includes(`DISABLED ${name}`) && !entries.includes(name))
      .map((name) => path.join(ALPHA, `DISABLED ${name}`));
    if (restore.length > 0) {
      const result = await invokeInApp<BulkResult>('bulk_toggle_mods', {
        gameId,
        paths: restore,
        enable: true,
        operationId: `qa-workspace-restore-${randomUUID()}`,
        intentRevision: null,
        expectedIdentities: null,
      });
      if (result.failures.length > 0) {
        throw new Error(`Could not restore owned test targets: ${JSON.stringify(result.failures)}`);
      }
      await reconcile(gameId);
      await browser.url('http://tauri.localhost/');
      await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: UI_TIMEOUT });
    }
    await gotoWorkspaceView('mods');
    const objectSearch = await $('[data-testid="object-list-search"] input');
    if ((await objectSearch.getValue()) !== '') await objectSearch.setValue('');
    const objectPanel = await $('[data-testid="object-list-panel"]');
    const allFilter = await objectPanel.$('button=All');
    if (await allFilter.isDisplayed()) await allFilter.click();
    await openObject(ALPHA);
    await click('[data-testid="view-grid"]');
    const modSearch = await $('[data-testid="mod-grid-search"] input');
    if ((await modSearch.getValue()) !== '') await modSearch.setValue('');
  }

  beforeEach(async function () {
    this.timeout(CASE_TIMEOUT);
    beforeEachFailure = null;
    try {
      await arrangeWorkspace();
    } catch (error: unknown) {
      beforeEachFailure =
        error instanceof Error ? error : new Error(`Workspace setup failed: ${String(error)}`);
      throw error;
    }
  });

  afterEach(async function () {
    const test = this.currentTest;
    if (!test) throw new Error('Workspace afterEach has no test result');
    const error = test.err?.message ?? beforeEachFailure?.message;
    const state = beforeEachFailure ? 'before_each_failed' : (test.state ?? 'not_run');
    console.info(
      `WORKSPACE_QA_CASE ${JSON.stringify({ title: test.title, state, duration_ms: test.duration, error })}`,
    );
    if (test.state !== 'failed' && !beforeEachFailure && test.state !== undefined) return;
    const directory = ARTIFACT_DIRECTORY;
    await fs.mkdir(directory, { recursive: true });
    const label = test.title.replace(/[^a-zA-Z0-9_-]/g, '_').slice(0, 120);
    const screenshot = path.join(directory, `${label}.png`);
    await browser.saveScreenshot(screenshot);
    const dom = await browser.execute(() => {
      const cloned = document.body.cloneNode(true) as HTMLBodyElement;
      cloned
        .querySelectorAll('script, style, input, textarea, kbd')
        .forEach((node) => node.remove());
      return {
        text: cloned.textContent?.replace(/\s+/g, ' ').slice(0, 6000),
        controls: Array.from(
          document.querySelectorAll('button, input, [role="button"], dialog[open]'),
        )
          .slice(0, 160)
          .map((node) => ({
            tag: node.tagName,
            role: node.getAttribute('role'),
            name: node.getAttribute('aria-label') ?? node.getAttribute('title'),
            labelledBy: node.getAttribute('aria-labelledby'),
            disabled: node.hasAttribute('disabled'),
            displayed:
              node instanceof HTMLElement &&
              node.getBoundingClientRect().width > 0 &&
              node.getBoundingClientRect().height > 0 &&
              getComputedStyle(node).visibility !== 'hidden',
            checked: node instanceof HTMLInputElement ? node.checked : undefined,
            text: node.tagName === 'BUTTON' ? node.textContent?.trim().slice(0, 120) : undefined,
          })),
      };
    });
    const nativeMetadata: Array<Record<string, unknown>> = [];
    for (const [object, mod] of [
      [ALPHA, 'Bulk A'],
      [ALPHA, 'Bulk B'],
      [ALPHA, 'Preview'],
    ] as const) {
      const filename = path.join(game.modsPath, object, mod, 'info.json');
      if (!(await exists(filename))) {
        nativeMetadata.push({ object, mod, present: false });
        continue;
      }
      const info: unknown = JSON.parse(await fs.readFile(filename, 'utf8'));
      if (typeof info !== 'object' || info === null) {
        throw new Error(`Fixture metadata is not an object: ${object}/${mod}`);
      }
      nativeMetadata.push({
        object,
        mod,
        present: true,
        pinned: 'is_pinned' in info ? info.is_pinned : null,
        favorite: 'is_favorite' in info ? info.is_favorite : null,
        versionMatchesEdit: 'version' in info && info.version === '2.1',
        emptyDescription: 'description' in info && info.description === '',
        descriptionLength:
          'description' in info && typeof info.description === 'string'
            ? info.description.length
            : null,
      });
    }
    await fs.writeFile(
      path.join(directory, `${label}.json`),
      JSON.stringify(
        { test: test.title, state, error, url: await browser.getUrl(), dom, nativeMetadata },
        null,
        2,
      ),
    );
    console.error(`WORKSPACE_QA_FAILURE_ARTIFACT ${screenshot}`);
  });

  after(() => {
    console.info(
      `WORKSPACE_QA_GAPS ${JSON.stringify({
        dependency_required: [
          'Native game/folder/file pickers',
          'Explorer and default INI editor',
          'External Mod Viewer',
          'Game/loader launch and global hotkeys',
          'Real clipboard image paste',
        ],
        not_executed: [
          'Positive mod/image recycle-bin deletion (fixture leaves would escape normal cleanup)',
          'Drag rectangle and pane resize gestures',
          'Bulk Move to Object and metadata/safety menus',
        ],
        observed_product_shape: 'Folder rename is inline; no RenameDialog component exists.',
      })}`,
    );
    if (game) scheduleMockGameRemoval(game);
    if (secondGame) scheduleMockGameRemoval(secondGame);
  });

  it('navigates Dashboard, Mods and Collections through the App Menu', async () => {
    for (const view of ['dashboard', 'collections', 'mods', 'dashboard', 'mods']) {
      await gotoWorkspaceView(view);
      await expect(
        await $(`[data-testid="dashboard-layout"][data-workspace-view="${view}"]`),
      ).toBeDisplayed();
    }
    await expect(await $('[data-testid="workspace-left"]')).toBeDisplayed();
    await expect(await $('[data-testid="workspace-main"]')).toBeDisplayed();
    await expect(await $('[data-testid="workspace-right"]')).toBeDisplayed();
  });

  it('selects a second game and restores the first without mixing object rows', async () => {
    await selectGame(OTHER_GAME, secondGameId);
    const secondObjects = await getObjects(secondGameId);
    expect(secondObjects).toHaveLength(1);
    await expect(await $(`[data-object-id="${secondObjects[0].id}"]`)).toBeDisplayed();
    await expect(await $(`[data-object-id="${objectIds[ALPHA]}"]`)).not.toExist();
    await selectGame(MAIN_GAME, gameId);
    await expect(await $(`[data-object-id="${objectIds[ALPHA]}"]`)).toBeDisplayed();
    await expect(await $(`[data-object-id="${secondObjects[0].id}"]`)).not.toExist();
    await selectGame(MAIN_GAME, gameId);
    expect((await getObjects(gameId)).map((object) => object.id).sort()).toEqual(
      Object.values(objectIds).sort(),
    );
  });

  it('filters object search including Unicode no-match and clears it through the UI', async () => {
    const search = await $('[data-testid="object-list-search"] input');
    await search.click();
    await search.setValue(BETA);
    await expect(await $(`[data-object-id="${objectIds[BETA]}"]`)).toBeDisplayed();
    await expect(await $(`[data-object-id="${objectIds[ALPHA]}"]`)).not.toExist();
    await search.setValue('不存在 QA missing');
    await expect(await $('[data-testid="object-list-panel"]')).toHaveText(
      expect.stringContaining('No results match your search'),
    );
    await click('//*[@data-testid="object-list-panel"]//button[normalize-space(.)="Clear Search"]');
    await expect(search).toHaveValue('');
    for (const id of Object.values(objectIds)) {
      await expect(await $(`[data-object-id="${id}"]`)).toBeDisplayed();
    }
  });

  it('changes object status filters and sort chips, then clears the filter', async () => {
    const panel = await $('[data-testid="object-list-panel"]');
    const showFilters = await panel.$('button[title="Show Filters"]');
    if (await showFilters.isDisplayed()) await showFilters.click();
    await (await panel.$('button=Disabled')).click();
    await browser.waitUntil(async () => (await panel.$$('[data-object-id]').length) === 0, {
      timeout: UI_TIMEOUT,
      timeoutMsg: 'Disabled object filter did not remove all enabled fixture rows',
    });
    await expect(panel).toHaveText(expect.stringContaining('0 objects'));
    await (await panel.$('button=Enabled')).click();
    await expect(await $(`[data-object-id="${objectIds[ALPHA]}"]`)).toBeDisplayed();
    await (await panel.$('button=New')).click();
    await expect(await $(`[data-object-id="${objectIds[BETA]}"]`)).toBeDisplayed();
    await (await panel.$('button=A–Z')).click();
    await (await panel.$('button=All')).click();
    await (await panel.$('button[title="Hide Filters"]')).click();
    await expect(await panel.$('button[title="Show Filters"]')).toHaveAttribute(
      'aria-pressed',
      'false',
    );
    await (await panel.$('button[title="Show Filters"]')).click();
    await expect(await $$('[data-testid="object-list-panel"] [data-object-id]')).toHaveLength(3);
  });

  it('searches mods, clears an empty result and preserves targets through list and sort changes', async () => {
    const search = await $('[data-testid="mod-grid-search"] input');
    await search.click();
    await search.setValue('Bulk');
    await card('Bulk A');
    await card('Bulk B');
    await expect(await $('[role="gridcell"][aria-label="Preview, enabled"]')).not.toExist();
    await search.setValue('不存在 QA missing');
    await click('//*[@data-testid="folder-grid"]//button[normalize-space(.)="Clear Search"]');
    await expect(search).toHaveValue('');
    await click('[data-testid="folder-sort"]');
    await click('//button[@role="option" and normalize-space(.)="Name (Z–A)"]');
    await expect(await $('[data-testid="folder-sort"]')).toHaveText('Name (Z–A)');
    await click('[data-testid="view-list"]');
    await expect(await $('[data-testid="view-list"]')).toHaveAttribute('aria-pressed', 'true');
    await expect(await $('[data-testid="folder-grid"]')).toHaveText(
      expect.stringContaining('Bulk A'),
    );
    await click('[data-testid="view-grid"]');
    await expect(await $('[data-testid="view-grid"]')).toHaveAttribute('aria-pressed', 'true');
    await card('Preview');
    await click('[data-testid="folder-sort"]');
    await click('//button[@role="option" and normalize-space(.)="Name (A–Z)"]');
  });

  it('switches a card and its preview against exact filesystem prefixes', async () => {
    await switchCard('Rapid');
    await assertPrefix(ALPHA, 'Rapid', false);
    await openPreview('Rapid');
    const previewSwitch = await $(
      '[data-testid="workspace-right"] input[type="checkbox"][aria-label]',
    );
    await previewSwitch.waitForClickable({ timeout: UI_TIMEOUT });
    await previewSwitch.click();
    await assertPrefix(ALPHA, 'Rapid', true);
    expect(
      await fs.readFile(path.join(game.modsPath, ALPHA, 'Rapid', 'qa-sentinel.txt'), 'utf8'),
    ).toBe(`${ALPHA}/Rapid`);
  });

  it('accounts for seven rapid rendered switch clicks and keeps the last intent', async () => {
    await openPreview('Rapid');
    const initial = await (await card('Rapid')).$('input[type="checkbox"][aria-label]');
    await expect(initial).toBeChecked();
    await browser.execute(() => {
      const state = window as Window & { __qaWorkspaceClicks?: boolean[] };
      state.__qaWorkspaceClicks = [];
      document.addEventListener('change', (event) => {
        const input = event.target;
        if (
          input instanceof HTMLInputElement &&
          input.type === 'checkbox' &&
          input.closest('[role="gridcell"]')?.querySelector('h3')?.title === 'Rapid'
        ) {
          state.__qaWorkspaceClicks?.push(input.checked);
        }
      });
    });
    for (let index = 0; index < 7; index += 1) await switchCard('Rapid');
    const requests = await browser.execute(
      () => (window as Window & { __qaWorkspaceClicks?: boolean[] }).__qaWorkspaceClicks ?? [],
    );
    expect(requests).toEqual([false, true, false, true, false, true, false]);
    console.info(
      `WORKSPACE_QA_RAPID ${JSON.stringify({ clicks: requests, final_enabled: false })}`,
    );
    await assertPrefix(ALPHA, 'Rapid', false);
    await switchCard('Rapid');
    await assertPrefix(ALPHA, 'Rapid', true);
  });

  it('selects two mods and runs bulk disable, enable and clear on the frozen selection', async () => {
    await browser.execute(() => {
      const state = window as Window & { __qaBulkClicks?: string[] };
      state.__qaBulkClicks = [];
      document.addEventListener('click', (event) => {
        const button =
          event.target instanceof Element
            ? event.target.closest('.bulk-action-bar--floating button')
            : null;
        if (button) state.__qaBulkClicks?.push(button.textContent?.trim() ?? '');
      });
    });
    await selectCard('Bulk A');
    await selectCard('Bulk B');
    const bar = await $('.bulk-action-bar--floating');
    await bar.waitForDisplayed({ timeout: UI_TIMEOUT });
    await expect(bar).toHaveText(expect.stringContaining('2'));
    await (await bar.$('button=Disable')).click();
    console.info('WORKSPACE_QA_BULK_STAGE disable_clicked');
    await assertPrefix(ALPHA, 'Bulk A', false);
    await assertPrefix(ALPHA, 'Bulk B', false);
    console.info('WORKSPACE_QA_BULK_STAGE disabled_verified');
    await expect(await (await card('Bulk A')).$('input:not([aria-label])')).toBeChecked();
    await (await bar.$('button=Enable')).click();
    console.info('WORKSPACE_QA_BULK_STAGE enable_clicked');
    const clicks = await browser.execute(
      () => (window as Window & { __qaBulkClicks?: string[] }).__qaBulkClicks ?? [],
    );
    console.info(`WORKSPACE_QA_BULK_CLICKS ${JSON.stringify(clicks)}`);
    expect(clicks).toEqual(['Disable', 'Enable']);
    await assertPrefix(ALPHA, 'Bulk A', true);
    await assertPrefix(ALPHA, 'Bulk B', true);
    console.info('WORKSPACE_QA_BULK_STAGE enabled_verified');
    await (await bar.$('button[aria-label="Clear selection"]')).click();
    await expect(bar).not.toExist();
    await expect(await (await card('Bulk A')).$('input:not([aria-label])')).not.toBeChecked();
    expect(await exists(path.join(game.modsPath, ALPHA, 'Preview'))).toBe(true);
  });

  it('round-trips Favorite, bulk Pin and Remove Pin on isolated targets', async () => {
    const favorite = await card('Bulk A');
    await favorite.moveTo();
    await (await favorite.$('button[title="Favorite"]')).click();
    await expect(await (await card('Bulk A')).$('button[title="Unfavorite"]')).toBeDisplayed();
    await (await (await card('Bulk A')).$('button[title="Unfavorite"]')).click();
    await expect(await (await card('Bulk A')).$('button[title="Favorite"]')).toExist();
    await selectCard('Bulk A');
    await selectCard('Bulk B');
    await expect(await $('.bulk-action-bar--floating')).toHaveText(expect.stringContaining('2'));
    await expect(await (await card('Bulk A')).$('input:not([aria-label])')).toBeChecked();
    await expect(await (await card('Bulk B')).$('input:not([aria-label])')).toBeChecked();
    await click('.bulk-action-bar--floating button[title="Pin to Object"]');
    const pinInfo = async (name: string): Promise<boolean> => {
      const info: unknown = JSON.parse(
        await fs.readFile(path.join(game.modsPath, ALPHA, name, 'info.json'), 'utf8'),
      );
      return (
        typeof info === 'object' && info !== null && 'is_pinned' in info && info.is_pinned === true
      );
    };
    await browser.waitUntil(async () => (await pinInfo('Bulk A')) && (await pinInfo('Bulk B')), {
      timeout: UI_TIMEOUT,
      timeoutMsg: 'Bulk Pin did not persist both fixture metadata flags',
    });
    await click('.bulk-action-bar--floating button[aria-label="More actions"]');
    await click(
      '//*[contains(concat(" ", normalize-space(@class), " "), " bulk-action-bar--floating ")]//button[normalize-space(.)="Remove Pin"]',
    );
    await browser.waitUntil(async () => !(await pinInfo('Bulk A')) && !(await pinInfo('Bulk B')), {
      timeout: UI_TIMEOUT,
      timeoutMsg: 'Bulk Remove Pin did not persist both fixture metadata flags',
    });
    await click('.bulk-action-bar--floating button[aria-label="Clear selection"]');
  });

  it('validates blank and duplicate folder names, cancels and creates a Unicode folder', async () => {
    const baseline = (await fs.readdir(path.join(game.modsPath, ALPHA))).sort();
    await click('[data-testid="add-folder"]');
    const dialog = await $('dialog[open][aria-labelledby="create-folder-title"]');
    const input = await dialog.$('input');
    const submit = await dialog.$('button[type="submit"]');
    await expect(submit).toBeDisabled();
    await expect(await $('#create-folder-validation')).toHaveText('Enter a folder name.');
    await input.setValue('  bulk a  ');
    await expect(submit).toBeDisabled();
    await expect(await $('#create-folder-validation')).toHaveText(
      'A folder with this name already exists here.',
    );
    await input.setValue('QA canceled folder');
    await dialogButton('Cancel');
    expect((await fs.readdir(path.join(game.modsPath, ALPHA))).sort()).toEqual(baseline);
    await click('[data-testid="add-folder"]');
    await (
      await $('dialog[open][aria-labelledby="create-folder-title"] input')
    ).setValue('  QA 变体 Ω  ');
    await dialogButton('Add folder');
    await browser.waitUntil(() => exists(path.join(game.modsPath, ALPHA, 'QA 变体 Ω')), {
      timeout: UI_TIMEOUT,
      timeoutMsg: 'Unicode folder was not created on disk',
    });
    await card('QA 变体 Ω');
    expect(await exists(path.join(game.modsPath, ALPHA, '  QA 变体 Ω  '))).toBe(false);
  });

  it('checks inline rename blank, cancel, normalized duplicate and Unicode success', async () => {
    await openObject(BETA);
    const directory = path.join(game.modsPath, BETA);
    const baseline = (await fs.readdir(directory)).sort();
    let input = await openRename('Rename Source');
    await replaceText(input, '   ');
    await browser.keys('Enter');
    await expect(await (await card('Rename Source')).$('h3 input')).not.toExist();
    expect((await fs.readdir(directory)).sort()).toEqual(baseline);
    input = await openRename('Rename Source');
    await replaceText(input, 'QA canceled rename');
    await browser.keys('Escape');
    expect((await fs.readdir(directory)).sort()).toEqual(baseline);
    input = await openRename('Rename Source');
    await replaceText(input, 'Shared');
    await browser.keys('Enter');
    await expect(await $('body')).toHaveText(expect.stringContaining('already exists'));
    expect((await fs.readdir(directory)).sort()).toEqual(baseline);
    await browser.keys('Escape');
    input = await openRename('Rename Source');
    await replaceText(input, '  QA 名称 Ω  ');
    await browser.keys('Enter');
    await browser.waitUntil(() => exists(path.join(directory, 'QA 名称 Ω')), {
      timeout: UI_TIMEOUT,
      timeoutMsg: 'Unicode rename did not reach disk',
    });
    await card('QA 名称 Ω');
    expect(await exists(path.join(directory, 'Rename Source'))).toBe(false);
    expect(await fs.readFile(path.join(directory, 'Shared', 'qa-sentinel.txt'), 'utf8')).toBe(
      `${BETA}/Shared`,
    );
  });

  it('cancels then confirms disabled-ancestor enable without changing own-disabled or same-name sentinels', async () => {
    await openObject(GAMMA);
    await (await card('Parent')).doubleClick();
    await card('Shared');
    await switchCard('Shared');
    let dialog = await $('dialog[open][aria-labelledby="workspace-parent-enable-title"]');
    await dialog.waitForDisplayed({ timeout: UI_TIMEOUT });
    await expect(dialog).toHaveText(expect.stringContaining('Own Sentinel'));
    await dialogButton('Cancel');
    expect(await exists(path.join(game.modsPath, GAMMA, 'DISABLED Parent', 'Shared'))).toBe(true);
    await switchCard('Shared');
    dialog = await $('dialog[open][aria-labelledby="workspace-parent-enable-title"]');
    await dialog.waitForDisplayed({ timeout: UI_TIMEOUT });
    const confirm = await dialog.$('button.btn-warning');
    await confirm.click();
    await browser.waitUntil(() => exists(path.join(game.modsPath, GAMMA, 'Parent', 'Shared')), {
      timeout: CASE_TIMEOUT,
      timeoutMsg: 'Confirmed ancestor enable did not rename Parent',
    });
    expect(await exists(path.join(game.modsPath, GAMMA, 'DISABLED Parent'))).toBe(false);
    expect(await exists(path.join(game.modsPath, GAMMA, 'Parent', 'DISABLED Own Sentinel'))).toBe(
      true,
    );
    expect(await exists(path.join(game.modsPath, BETA, 'Shared'))).toBe(true);
    expect(
      await fs.readFile(path.join(game.modsPath, BETA, 'Shared', 'qa-sentinel.txt'), 'utf8'),
    ).toBe(`${BETA}/Shared`);
    await expect(
      await (await card('Shared')).$('input[type="checkbox"][aria-label]'),
    ).toBeChecked();
  });

  it('renders a true enabled/disabled name collision as read-only without conflating hash conflicts', async () => {
    const enabled = await addMockMod(game, GAMMA, 'Collision');
    const disabled = await addMockMod(game, GAMMA, 'DISABLED Collision');
    await fs.writeFile(path.join(enabled, 'qa-sentinel.txt'), 'enabled collision sentinel');
    await fs.writeFile(path.join(disabled, 'qa-sentinel.txt'), 'disabled collision sentinel');
    await fs.writeFile(
      path.join(enabled, 'mod.ini'),
      '[TextureOverrideCollisionA]\nhash = 90000001\n',
    );
    await fs.writeFile(
      path.join(disabled, 'mod.ini'),
      '[TextureOverrideCollisionB]\nhash = 90000002\n',
    );
    try {
      await reconcile(gameId);
      await browser.url('http://tauri.localhost/');
      await gotoWorkspaceView('mods');
      await openObject(GAMMA);
      const collision = await card('Collision');
      await expect(await collision.$('[aria-label="Name Conflict"]')).toBeDisplayed();
      await expect(await $('[data-testid="folder-conflict-banner"]')).toHaveText(
        expect.stringContaining('folder name conflict'),
      );
      // Collision switches remain actions: native no-overwrite rejection opens the resolution UI.
      await expect(await collision.$('input[type="checkbox"][aria-label]')).toBeEnabled();
      await collision.click();
      await expect(await $('[data-testid="workspace-right"]')).toHaveText(
        expect.stringContaining('Folder name conflict'),
      );
      console.info(
        `WORKSPACE_QA_COLLISION ${JSON.stringify({
          physical_name_conflict: true,
          shared_hash_advisory_present: await collision
            .$('[aria-label="Shared Hash"]')
            .isExisting(),
        })}`,
      );
      await switchCard('Collision');
      const resolution = await $('dialog[open][aria-labelledby="folder-conflict-manager-title"]');
      await resolution.waitForDisplayed({ timeout: UI_TIMEOUT });
      await expect(resolution).toHaveText(expect.stringContaining('Collision'));
      await (await resolution.$('button[aria-label].btn-circle')).click();
      expect(await fs.readFile(path.join(enabled, 'qa-sentinel.txt'), 'utf8')).toBe(
        'enabled collision sentinel',
      );
      expect(await fs.readFile(path.join(disabled, 'qa-sentinel.txt'), 'utf8')).toBe(
        'disabled collision sentinel',
      );
    } finally {
      // Resolve only this test's duplicate fixture so subsequent independent UI cases can proceed.
      await fs.rename(disabled, path.join(game.modsPath, GAMMA, 'DISABLED Collision Resolved'));
      await reconcile(gameId);
    }
  });

  it('edits metadata through the preview and persists Unicode and empty description', async () => {
    await openPreview('Preview');
    await click('button[title="Edit Metadata"]');
    await (await $('input[aria-label="Author"]')).setValue('QA 作者 Ω');
    await (await $('input[aria-label="Mod version"]')).setValue('2.1');
    await (await $('textarea[aria-label="Description"]')).setValue('QA multiline\n第二行');
    await browser.waitUntil(
      async () => {
        const info: unknown = JSON.parse(
          await fs.readFile(path.join(previewPath, 'info.json'), 'utf8'),
        );
        return (
          typeof info === 'object' &&
          info !== null &&
          'author' in info &&
          info.author === 'QA 作者 Ω' &&
          'description' in info &&
          info.description === 'QA multiline\n第二行'
        );
      },
      { timeout: UI_TIMEOUT, timeoutMsg: 'Metadata edit did not autosave to info.json' },
    );
    const description = $('textarea[aria-label="Description"]');
    await replaceText(description, '');
    await expect(description).toHaveValue('');
    await browser.waitUntil(
      async () => {
        const info = await invokeInApp<{ description: string | null; version: string }>(
          'read_mod_info',
          { gameId, folderPath: previewPath },
        );
        return !info.description && info.version === '2.1';
      },
      { timeout: UI_TIMEOUT, timeoutMsg: 'Empty description did not persist' },
    );
    await click('//*[@data-testid="workspace-right"]//button[normalize-space(.)="Done"]');
    await expect(await $('input[aria-label="Author"]')).not.toExist();
  });

  it('preserves an unsaved metadata draft through its own disk toggle and saves at the renamed path', async () => {
    await openPreview('Preview');
    await click('button[title="Edit Metadata"]');
    const draft = 'QA pending toggle';
    await replaceText($('textarea[aria-label="Description"]'), draft);
    const before: unknown = JSON.parse(
      await fs.readFile(path.join(previewPath, 'info.json'), 'utf8'),
    );
    expect(
      typeof before === 'object' && before !== null && 'description' in before
        ? before.description
        : null,
    ).not.toBe(draft);
    await switchCard('Preview');
    const disabledPath = path.join(game.modsPath, ALPHA, 'DISABLED Preview');
    await browser.waitUntil(() => exists(disabledPath), {
      timeout: UI_TIMEOUT,
      timeoutMsg: 'Own preview mod did not disable on disk',
    });
    await expect(await $('textarea[aria-label="Description"]')).toHaveValue(draft);
    await browser.waitUntil(
      async () => {
        const info: unknown = JSON.parse(
          await fs.readFile(path.join(disabledPath, 'info.json'), 'utf8'),
        );
        return (
          typeof info === 'object' &&
          info !== null &&
          'description' in info &&
          info.description === draft
        );
      },
      {
        timeout: UI_TIMEOUT,
        timeoutMsg: 'Preserved metadata draft did not save to disabled directory',
      },
    );
    await switchCard('Preview');
    await browser.waitUntil(() => exists(previewPath), { timeout: UI_TIMEOUT });
    const saved: unknown = JSON.parse(
      await fs.readFile(path.join(previewPath, 'info.json'), 'utf8'),
    );
    expect(
      typeof saved === 'object' && saved !== null && 'description' in saved
        ? saved.description
        : null,
    ).toBe(draft);
  });

  it('validates INI key edits, cancels an unsaved transition, discards and saves through rendered controls', async () => {
    await openPreview('Preview');
    const original = await fs.readFile(path.join(previewPath, 'mod.ini'), 'utf8');
    await click('button[title="Edit keybinds"]');
    const expand = await $(
      '//*[@data-testid="workspace-right"]//button[@aria-expanded and contains(normalize-space(.), "mod.ini")]',
    );
    if ((await expand.getAttribute('aria-expanded')) !== 'true') await expand.click();
    const key = await $('input[placeholder="Enter key..."]');
    await key.waitForDisplayed({ timeout: UI_TIMEOUT });
    await key.setValue('!!! invalid QA key !!!');
    await expect(await $('button[title="Save INI changes"]')).toBeDisabled();
    expect(await fs.readFile(path.join(previewPath, 'mod.ini'), 'utf8')).toBe(original);
    await key.setValue('VK_F7');
    await expect(await $('button[title="Save INI changes"]')).toBeEnabled();
    await (await card('Bulk A')).click();
    await expect(await $('dialog[open]')).toHaveText(expect.stringContaining('Unsaved Changes'));
    await dialogButton('Cancel');
    await expect(
      await $('[data-testid="workspace-right"] input[aria-label="Rename mod"]'),
    ).toHaveValue('Preview');
    await expect(key).toHaveValue('VK_F7');
    expect(await fs.readFile(path.join(previewPath, 'mod.ini'), 'utf8')).toBe(original);
    await click('button[title="Discard INI changes"]');
    expect(await fs.readFile(path.join(previewPath, 'mod.ini'), 'utf8')).toBe(original);
    await click('button[title="Edit keybinds"]');
    await (await $('input[placeholder="Enter key..."]')).setValue('VK_F8');
    await click('button[title="Save INI changes"]');
    await browser.waitUntil(
      async () =>
        (await fs.readFile(path.join(previewPath, 'mod.ini'), 'utf8')).includes('key = VK_F8'),
      {
        timeout: UI_TIMEOUT,
        timeoutMsg: 'Rendered INI Save did not persist VK_F8',
      },
    );
    expect(await fs.readFile(path.join(previewPath, 'mod.ini'), 'utf8')).toContain(
      'hash = 00000f01',
    );
  });

  it('navigates gallery images, wraps fullscreen navigation and cancels image deletion', async () => {
    await openPreview('Preview');
    const images = await invokeInApp<string[]>('list_mod_preview_images', {
      gameId,
      folderPath: previewPath,
    });
    expect(images).toHaveLength(2);
    await click('button[aria-label="Go to image 2"]');
    await expect(await $('button[aria-label="Go to image 2"]')).toHaveAttribute(
      'aria-current',
      'true',
    );
    await click('button[aria-label="Open fullscreen preview"]');
    const fullscreen = await $('dialog[open][aria-label="Fullscreen"]');
    await fullscreen.waitForDisplayed({ timeout: UI_TIMEOUT });
    const image = await fullscreen.$('img[alt="Preview image"]');
    const initial = await image.getAttribute('src');
    if (!initial) throw new Error('Fullscreen preview image has no source');
    await (await fullscreen.$('button[aria-label="actions.next"]')).click();
    await browser.waitUntil(async () => (await image.getAttribute('src')) !== initial, {
      timeout: UI_TIMEOUT,
      timeoutMsg: 'Next gallery button did not wrap to first image',
    });
    await (await fullscreen.$('button[aria-label="actions.prev"]')).click();
    await expect(image).toHaveAttribute('src', initial);
    await (await fullscreen.$('button[aria-label="Close"]')).click();
    await expect(fullscreen).not.toExist();
    await click('button[aria-label="Preview image actions"]');
    await click('//button[@role="menuitem" and normalize-space(.)="Delete current preview image"]');
    await dialogButton('Cancel');
    expect(
      await invokeInApp<string[]>('list_mod_preview_images', { gameId, folderPath: previewPath }),
    ).toEqual(images);
  });

  it('rejects a blank collection name, cancels and saves a Unicode collection from current state', async () => {
    await gotoWorkspaceView('collections');
    const baseline = await listCollections();
    await click('button=Save Current State');
    const input = await $('dialog[open][aria-labelledby="save-collection-title"] input');
    await input.setValue('   ');
    await expect(await $('dialog[open] button[type="submit"]')).toBeDisabled();
    await input.setValue('QA canceled collection');
    await (await $('dialog[open] .card-body button')).click();
    expect(await listCollections()).toEqual(baseline);
    await click('button=Save Current State');
    await (
      await $('dialog[open][aria-labelledby="save-collection-title"] input')
    ).setValue('  QA 集合 Ω  ');
    await dialogButton('Save Collection');
    await collectionRow('QA 集合 Ω');
    const saved = (await listCollections()).find((collection) => collection.name === 'QA 集合 Ω');
    if (!saved) throw new Error('UI-created Unicode collection is missing from native read model');
    const state = await runtime();
    const snapshot = await storedPreview(saved.id);
    expect(snapshot.projected_state.active_roots.map((root) => root.root_key).sort()).toEqual(
      state.projected_state.active_roots.map((root) => root.root_key).sort(),
    );
  });

  it('renames a stored collection, cancels and handles a blank inline name without changing membership', async () => {
    const seeded = await seedCollection('QA rename');
    const snapshot = await storedPreview(seeded.id);
    await browser.url('http://tauri.localhost/');
    await gotoWorkspaceView('collections');
    let row = await collectionRow(seeded.name);
    await row.moveTo();
    await (await row.$('button[title="Rename"]')).click();
    await (await row.$('input')).setValue('QA canceled stored name');
    await (await row.$('input + button + button')).click();
    expect((await listCollections()).find((item) => item.id === seeded.id)?.name).toBe(seeded.name);
    await (await row.$('button[title="Rename"]')).click();
    await (await row.$('input')).setValue('   ');
    await (await row.$('input + button')).click();
    expect((await listCollections()).find((item) => item.id === seeded.id)?.name).toBe(seeded.name);
    await (await row.$('button[title="Rename"]')).click();
    await (await row.$('input')).setValue('  QA 保存名称 Ω  ');
    await (await row.$('input + button')).click();
    row = await collectionRow('QA 保存名称 Ω');
    await expect(row).toBeDisplayed();
    expect((await storedPreview(seeded.id)).projected_state).toEqual(snapshot.projected_state);
  });

  it('distinguishes current from saved membership, cancels apply and confirms exact disk restoration', async () => {
    const baseline = await seedCollection('QA apply baseline');
    const saved = await invokeInApp<CollectionSummary>('create_collection', {
      gameId,
      name: `QA apply ${randomUUID()}`,
      saveMode: 'clone_snapshot',
      sourceCollectionId: baseline.id,
    });
    expect((await runtime()).active_collection_id).toBe(baseline.id);
    const savedState = await storedPreview(saved.id);
    const rootBefore = savedState.projected_state.active_roots.find(
      (root) => root.display_name === 'Rapid',
    );
    if (!rootBefore) throw new Error('Apply fixture snapshot has no Rapid member');
    await switchCard('Rapid');
    await assertPrefix(ALPHA, 'Rapid', false);
    await browser.waitUntil(
      async () =>
        !(await runtime()).projected_state.active_roots.some(
          (root) => root.root_key === rootBefore.root_key,
        ),
      { timeout: UI_TIMEOUT, timeoutMsg: 'Current membership still contains disabled Rapid' },
    );
    expect(
      (await storedPreview(saved.id)).projected_state.active_roots.map((root) => root.root_key),
    ).toContain(rootBefore.root_key);
    await gotoWorkspaceView('collections');
    const row = await collectionRow(saved.name);
    await (await row.$('button=Apply')).click();
    await $('[data-testid="modal-apply-btn"]').waitForEnabled({ timeout: UI_TIMEOUT });
    await click('//button[@role="tab" and normalize-space(.)="Changes only"]');
    const applyDialog = await $('//*[@data-testid="modal-apply-btn"]/ancestor::dialog');
    await expect(applyDialog).toHaveText(expect.stringContaining('Rapid'));
    await click('//button[@role="tab" and normalize-space(.)="Full collection"]');
    await (await applyDialog.$('button=Cancel')).click();
    expect(await exists(path.join(game.modsPath, ALPHA, 'DISABLED Rapid'))).toBe(true);
    await (await (await collectionRow(saved.name)).$('button=Apply')).click();
    await click('[data-testid="modal-apply-btn"]');
    await browser.waitUntil(() => exists(path.join(game.modsPath, ALPHA, 'Rapid')), {
      timeout: CASE_TIMEOUT,
      timeoutMsg: 'Confirmed collection apply did not restore Rapid',
    });
    await browser.waitUntil(async () => (await runtime()).runtime_status === 'clean', {
      timeout: CASE_TIMEOUT,
      timeoutMsg: 'Confirmed collection did not settle to clean runtime',
    });
    const final = await runtime();
    expect(final.active_collection_id).toBe(saved.id);
    expect(final.projected_state.active_roots.map((root) => root.root_key).sort()).toEqual(
      savedState.projected_state.active_roots.map((root) => root.root_key).sort(),
    );
    for (const root of final.projected_state.active_roots) {
      const sourcePath = path.isAbsolute(root.source_path)
        ? root.source_path
        : path.join(game.modsPath, root.source_path);
      const relative = path.relative(game.modsPath, sourcePath);
      expect(relative.startsWith('..') || path.isAbsolute(relative)).toBe(false);
      expect(await exists(sourcePath)).toBe(true);
      expect(
        normalizePath(root.source_path)
          .split('/')
          .some((segment) => segment.startsWith('disabled ')),
      ).toBe(false);
    }
  });

  it('cancels collection deletion then deletes only the owned E2E row and preserves mod files', async () => {
    const seeded = await seedCollection('QA delete');
    const beforeFiles = (await fs.readdir(path.join(game.modsPath, ALPHA))).sort();
    await browser.url('http://tauri.localhost/');
    await gotoWorkspaceView('collections');
    let row = await collectionRow(seeded.name);
    await (await row.$('button[title="Delete collection"]')).click();
    await dialogButton('Cancel');
    expect((await listCollections()).some((item) => item.id === seeded.id)).toBe(true);
    row = await collectionRow(seeded.name);
    await (await row.$('button[title="Delete collection"]')).click();
    await dialogButton('Delete');
    await browser.waitUntil(
      async () => !(await listCollections()).some((item) => item.id === seeded.id),
      { timeout: UI_TIMEOUT, timeoutMsg: 'UI collection delete did not remove its E2E row' },
    );
    await expect(row).not.toExist();
    expect((await fs.readdir(path.join(game.modsPath, ALPHA))).sort()).toEqual(beforeFiles);
  });
});
