import { $, browser, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
import {
  addMockMod,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { gotoWorkspaceView, seedGamesAndOpenDashboard } from '../support/app.js';
import { getObjects, reconcile, waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';
import type { AppSettings } from '../../../src/shared/api/tauri/bindings.gen.js';

const ROOT = path.resolve('landing/.screenshots');
const THUMBNAILS = path.resolve('tests/e2e/fixtures/landing-thumbnails');
const WAIT = 30_000;
const OBJECTS = [
  'Aster',
  'Boreal',
  'Cinder',
  'Dahlia',
  'Ember',
  'Fable',
  'Gale',
  'Halo',
  'Iris',
  'Juno',
  'Kestrel',
  'Lumen',
  'Mira',
  'Nova',
  'Orion',
  'Piper',
  'Quartz',
  'Riven',
  'Solace',
  'Vesper',
] as const;
const CHARACTER_THUMBNAILS = [
  'cobalt-tech.webp',
  'plum-coat.webp',
  'silver-tailored.webp',
  'copper-street.webp',
  'forest-layered.webp',
  'pink-tailored.webp',
  'auburn-layered.webp',
  'braided-cobalt.webp',
] as const;
const ASTER_THUMBNAILS = {
  'Aster Signature': 'ivory-noir.webp',
  'After Dark': 'aster-after-dark.webp',
  'Auburn Edit': 'aster-weekend-form.webp',
  'Blue Hour': 'aster-blue-hour.webp',
  'Braided Cobalt': 'aster-soft-armor.webp',
  'City Formal': 'aster-city-formal.webp',
  'Classic Line': 'aster-classic-line.webp',
  'Cobalt Tech': 'aster-cobalt-tech.webp',
  'Copper Street': 'aster-copper-street.webp',
  'Evening Edit': 'aster-evening-edit.webp',
  'Forest Layered': 'aster-forest-layered.webp',
  'Gallery Look': 'aster-gallery-look.webp',
  'Ivory Noir': 'aster-silver-tailored.webp',
  'Late Shift': 'aster-winter-cut.webp',
  'Modern Uniform': 'aster-plum-atelier.webp',
  'Pink Studio': 'aster-pink-studio.webp',
  'Plum Atelier': 'aster-plum-atelier.webp',
  'Silver Tailored': 'aster-silver-tailored.webp',
  'Midnight Coat': 'aster-after-dark.webp',
  'Soft Armor': 'aster-soft-armor.webp',
  'Studio Layers': 'aster-weekend-form.webp',
  'Winter Cut': 'aster-winter-cut.webp',
  'Weekend Form': 'aster-weekend-form.webp',
  'Night Walk': 'aster-blue-hour.webp',
  'Second Season': 'aster-forest-layered.webp',
  'Studio Variant': 'aster-soft-armor.webp',
  'Evening Variant': 'aster-pink-studio.webp',
  'Travel Variant': 'aster-copper-street.webp',
  'Winter Variant': 'aster-winter-cut.webp',
} as const;
const FASHIONS = [
  'Ivory Noir',
  'Cobalt Tech',
  'Plum Atelier',
  'Silver Tailored',
  'Copper Street',
  'Forest Layered',
  'Pink Studio',
  'Auburn Edit',
  'Braided Cobalt',
  'Midnight Coat',
  'City Formal',
  'Blue Hour',
  'Soft Armor',
  'After Dark',
  'Studio Layers',
  'Winter Cut',
  'Classic Line',
  'Late Shift',
  'Modern Uniform',
  'Evening Edit',
  'Weekend Form',
  'Gallery Look',
  'Night Walk',
  'Second Season',
] as const;
const SAMPLE_KEYBINDINGS = new Map([
  ['After Dark', 'F6'],
  ['Cobalt Tech', 'F9'],
  ['City Formal', 'F10'],
  ['Gallery Look', 'F11'],
]);
const NESTED_VARIANTS = [
  'Studio Variant',
  'Evening Variant',
  'Travel Variant',
  'Winter Variant',
] as const;

interface CollectionSummary {
  id: string;
  name: string;
}

async function addFashionMod(
  game: MockGame,
  object: string,
  name: string,
  ordinal: number,
  thumbnailFile: string,
): Promise<string> {
  const folder = await addMockMod(game, object, name);
  await fs.writeFile(
    path.join(folder, 'mod.ini'),
    `[TextureOverrideLanding${ordinal}]\nhash = ${ordinal.toString(16).padStart(16, '0')}\n`,
  );
  await fs.copyFile(path.join(THUMBNAILS, thumbnailFile), path.join(folder, 'preview.webp'));
  await fs.writeFile(
    path.join(folder, 'info.json'),
    JSON.stringify({ author: 'EMMM Sample Library', description: 'Original sample fashion mod' }),
  );
  return folder;
}

async function waitForImages(): Promise<void> {
  await browser.waitUntil(
    () =>
      browser.execute(() => {
        const images = Array.from(document.querySelectorAll('img')).filter(
          (image) => image.getBoundingClientRect().width > 0,
        );
        return images.every((image) => image.complete && image.naturalWidth > 0);
      }),
    {
      timeout: WAIT,
      interval: 150,
      timeoutMsg: 'Visible screenshot images did not finish loading',
    },
  );
  await browser.executeAsync((done: () => void) => {
    requestAnimationFrame(() => requestAnimationFrame(() => done()));
  });
}

async function waitForGridImages(count: number): Promise<void> {
  await browser.waitUntil(
    () =>
      browser.execute(
        (minimum) =>
          Array.from(document.querySelectorAll('[data-testid="folder-grid"] [role="gridcell"] img'))
            .filter((image): image is HTMLImageElement => image instanceof HTMLImageElement)
            .filter((image) => image.complete && image.naturalWidth > 0).length >= minimum,
        count,
      ),
    { timeout: WAIT, interval: 200, timeoutMsg: `Fewer than ${count} mod thumbnails loaded` },
  );
}

async function capture(name: string): Promise<void> {
  await waitForImages();
  await fs.mkdir(ROOT, { recursive: true });
  const screenshot = Buffer.from(await browser.takeScreenshot(), 'base64');
  const target = path.join(ROOT, `${name}.png`);
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      await fs.writeFile(target, screenshot);
      return;
    } catch (error) {
      if (attempt === 2) throw error;
      await new Promise((resolve) => setTimeout(resolve, 200));
    }
  }
}

async function replaceLocalUserInVisiblePaths(fixtureRoot: string): Promise<void> {
  const localUser = /[\\/]Users[\\/]([^\\/]+)/i.exec(fixtureRoot)?.[1];
  if (!localUser) throw new Error(`Cannot identify local user in fixture path: ${fixtureRoot}`);
  const replacements = await browser.execute((userName) => {
    const nodes = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    let count = 0;
    while (nodes.nextNode()) {
      const text = nodes.currentNode.nodeValue;
      if (text?.includes(userName)) {
        nodes.currentNode.nodeValue = text.replaceAll(userName, 'Sample');
        count += 1;
      }
    }
    return count;
  }, localUser);
  if (replacements === 0) throw new Error('No local user paths were replaced before capture');
}

async function visibleCount(selector: string): Promise<number> {
  return browser.execute(
    (query) =>
      Array.from(document.querySelectorAll(query)).filter((element) => {
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return (
          rect.width > 0 &&
          rect.height > 0 &&
          rect.top < window.innerHeight &&
          rect.bottom > 0 &&
          style.visibility !== 'hidden'
        );
      }).length,
    selector,
  );
}

describe('Opt-in native landing screenshots', function () {
  this.timeout(180_000);
  let game: MockGame;
  const dashboardGames: MockGame[] = [];
  let gameId = '';
  let inbox = '';
  let firstModPath = '';
  let unsafeModPath = '';
  let featuredModPath = '';
  const switchModPaths: string[] = [];

  before(async function () {
    this.timeout(180_000);
    game = await createMockGame('LandingCapture');
    for (const [label, total] of [
      ['Archive Library', 12],
      ['Test Bench', 7],
    ] as const) {
      const dashboardGame = await createMockGame(`Landing${label.replace(' ', '')}`);
      for (let index = 1; index <= total; index += 1) {
        await addMockMod(dashboardGame, label.replace(' ', ''), `Look ${index}`);
      }
      dashboardGames.push(dashboardGame);
    }
    inbox = path.join(game.fixtureRoot, 'Inbox');
    await fs.mkdir(inbox);

    let ordinal = 1;
    for (const object of OBJECTS) {
      const thumbnailFile =
        object === 'Aster'
          ? ASTER_THUMBNAILS['Aster Signature']
          : CHARACTER_THUMBNAILS[(ordinal - 2) % CHARACTER_THUMBNAILS.length];
      const folder = await addFashionMod(
        game,
        object,
        `${object} Signature`,
        ordinal,
        thumbnailFile,
      );
      if (object === 'Aster') firstModPath = folder;
      ordinal += 1;
    }
    for (const name of FASHIONS) {
      const folder = await addFashionMod(game, 'Aster', name, ordinal, ASTER_THUMBNAILS[name]);
      if (name === 'Ivory Noir') unsafeModPath = folder;
      if (name === 'Blue Hour' || name === 'Copper Street') switchModPaths.push(folder);
      if (name === 'After Dark') featuredModPath = folder;
      const key = SAMPLE_KEYBINDINGS.get(name);
      if (key) {
        await fs.appendFile(
          path.join(folder, 'mod.ini'),
          `\n[Constants]\nglobal $outfit = 0\n\n[KeyOutfitCycle]\nkey = ${key}\ntype = cycle\n$outfit = 0,1,2\n`,
        );
      }
      ordinal += 1;
    }
    for (const name of NESTED_VARIANTS) {
      await addFashionMod(
        game,
        path.join('Aster', 'Capsule Wardrobe'),
        name,
        ordinal,
        ASTER_THUMBNAILS[name],
      );
      ordinal += 1;
    }
    for (const name of ['Aster - Ivory Edit', 'Boreal - Copper Edit', 'Blue Edit', 'After Dark']) {
      const source = path.join(inbox, name);
      await fs.mkdir(source);
      await fs.writeFile(
        path.join(source, 'mod.ini'),
        `[TextureOverrideInbox${ordinal}]\nhash = ${ordinal.toString(16).padStart(16, '0')}\n`,
      );
      await fs.copyFile(
        path.join(
          THUMBNAILS,
          name.startsWith('Aster') || name === 'After Dark'
            ? ASTER_THUMBNAILS['Ivory Noir']
            : CHARACTER_THUMBNAILS[ordinal % CHARACTER_THUMBNAILS.length],
        ),
        path.join(source, 'preview.webp'),
      );
      ordinal += 1;
    }
    await addMockMod(game, 'Vesper', 'Archive Copy A');
    await addMockMod(game, 'Vesper', 'Archive Copy B');

    const [archiveGame, testBenchGame] = dashboardGames;
    if (!archiveGame || !testBenchGame) throw new Error('Dashboard sample games were not created');
    [gameId] = await seedGamesAndOpenDashboard(
      [
        { game, name: 'Studio Library' },
        { game: archiveGame, name: 'Archive Library' },
        { game: testBenchGame, name: 'Test Bench' },
      ],
      WAIT,
    );
    await reconcile(gameId);
    const settings = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', {
      settings: {
        ...settings,
        games: settings.games.map((entry) =>
          entry.id === gameId
            ? { ...entry, ready_to_move_path: inbox, game_exe: game.exePath }
            : entry,
        ),
      },
    });
    await browser.setWindowSize(1600, 1000);
    console.info(
      'LANDING_VIEWPORT',
      await browser.execute(() => ({
        width: window.innerWidth,
        height: window.innerHeight,
      })),
    );
    await waitForWorkspaceCoreReady(gameId, WAIT);
    await browser.waitUntil(async () => (await getObjects(gameId)).length >= OBJECTS.length, {
      timeout: WAIT,
      interval: 250,
      timeoutMsg: 'Sample Object List did not finish indexing',
    });
  });

  after(() => {
    if (game) scheduleMockGameRemoval(game);
    for (const dashboardGame of dashboardGames) scheduleMockGameRemoval(dashboardGame);
  });

  it('captures a dense workspace, nested folder, and Paste Thumbnail control', async () => {
    await gotoWorkspaceView('mods');
    const aster = (await getObjects(gameId)).find((object) => object.name === 'Aster');
    if (!aster) throw new Error('Aster was not indexed');
    const row = await $(`[data-object-id="${aster.id}"]`);
    await row.waitForClickable({ timeout: WAIT });
    await row.click();
    await $('[data-testid="folder-grid"]').waitForDisplayed({ timeout: WAIT });
    await $('[data-testid="view-grid"]').click();
    await browser.waitUntil(
      async () => (await visibleCount('[data-testid="folder-grid"] [role="gridcell"]')) >= 9,
      { timeout: WAIT, interval: 150, timeoutMsg: 'Fewer than nine mod cards are visible' },
    );
    const visibleObjects = await visibleCount('[data-object-id]');
    console.info('LANDING_WORKSPACE_VISIBLE_OBJECTS', visibleObjects);
    expect(visibleObjects).toBeGreaterThanOrEqual(8);
    const previewCard = await $(
      '//*[@data-testid="folder-grid"]//*[@role="gridcell"][.//h3[@title="After Dark"]]',
    );
    await previewCard.click();
    await $('button[aria-label="Preview image actions"]').waitForDisplayed({ timeout: WAIT });
    await capture('workspace-library');

    const pack = await $(
      '//*[@data-testid="folder-grid"]//*[@role="gridcell"][.//h3[@title="Capsule Wardrobe"]]',
    );
    await pack.waitForDisplayed({ timeout: WAIT });
    await pack.doubleClick();
    await $(
      '//*[@data-testid="folder-grid"]//*[@role="gridcell"][.//h3[@title="Studio Variant"]]',
    ).waitForDisplayed({ timeout: WAIT });
    await waitForGridImages(4);
    await capture('nested-subfolders');

    await row.click();
    const card = await $(
      '//*[@data-testid="folder-grid"]//*[@role="gridcell"][.//h3[@title="After Dark"]]',
    );
    await card.waitForDisplayed({ timeout: WAIT });
    await card.click();
    await $('button[aria-label="Preview image actions"]').click();
    await $(
      '//*[@role="menuitem" and normalize-space(.)="Paste image from clipboard"]',
    ).waitForDisplayed({
      timeout: WAIT,
    });
    await capture('paste-thumbnail');
    await browser.keys('Escape');

    const healthDetails = await $('//button[normalize-space(.)="Details"]');
    await healthDetails.scrollIntoView();
    await healthDetails.waitForClickable({ timeout: WAIT });
    await healthDetails.click();
    const healthDialog = await $('//dialog[.//h4[normalize-space(.)="Details"]]');
    await healthDialog.waitForDisplayed({ timeout: WAIT });
    await healthDialog.$('button[role="tab"]*=Controls').click();
    await capture('mod-health-details');
    await healthDialog.$('.//button[normalize-space(.)="Close"]').click();
    await healthDialog.waitForDisplayed({ reverse: true, timeout: WAIT });

    await invokeInApp('bulk_toggle_mods', {
      gameId,
      paths: switchModPaths,
      enable: false,
      operationId: `landing-switch-off-${randomUUID()}`,
      intentRevision: null,
      expectedIdentities: null,
    });
    await reconcile(gameId);
    await browser.refresh();
    await waitForWorkspaceCoreReady(gameId, WAIT);
    await gotoWorkspaceView('mods');
    await $(`[data-object-id="${aster.id}"]`).click();
    await $('[data-testid="folder-grid"]').waitForDisplayed({ timeout: WAIT });
    await capture('switch-controls');
    for (const name of ['Blue Hour', 'Copper Street']) {
      const card = await $(
        `//*[@data-testid="folder-grid"]//*[@role="gridcell"][.//h3[@title="${name}"]]`,
      );
      await card.$('input[type="checkbox"]').click();
    }
    await browser.waitUntil(
      async () =>
        (await visibleCount('[data-testid="folder-grid"] input[type="checkbox"]:checked')) >= 2,
      { timeout: WAIT, timeoutMsg: 'Bulk mod selection did not become visible' },
    );
    await capture('switch-bulk-actions');
    const selectedCard = await $(
      '//*[@data-testid="folder-grid"]//*[@role="gridcell"][.//h3[@title="Blue Hour"]]',
    );
    await selectedCard.click({ button: 'right' });
    await $('//*[normalize-space(.)="Pin Selected"]').waitForDisplayed({ timeout: WAIT });
    await capture('pin-mod-menu');
    await browser.keys('Escape');
    for (const name of ['Blue Hour', 'Copper Street']) {
      const card = await $(
        `//*[@data-testid="folder-grid"]//*[@role="gridcell"][.//h3[@title="${name}"]]`,
      );
      await card.$('input[type="checkbox"]').click();
    }
    await invokeInApp('bulk_toggle_mods', {
      gameId,
      paths: switchModPaths,
      enable: true,
      operationId: `landing-switch-on-${randomUUID()}`,
      intentRevision: null,
      expectedIdentities: null,
    });

    await invokeInApp('pin_object', { id: aster.id, pin: true });
    await invokeInApp('bulk_toggle_favorite', {
      gameId,
      folderPaths: [featuredModPath],
      favorite: true,
    });
    await reconcile(gameId);
    await browser.refresh();
    await waitForWorkspaceCoreReady(gameId, WAIT);
    await gotoWorkspaceView('mods');
    await $(`[data-object-id="${aster.id}"]`).click();
    await $('[data-testid="folder-grid"]').waitForDisplayed({ timeout: WAIT });
    await $(
      '//*[@data-testid="folder-grid"]//*[@role="gridcell"][.//h3[@title="After Dark"]]',
    ).click();
    await capture('favorites-filters');
    const objectSearch = await $('[data-testid="object-list-search"] input[type="search"]');
    await objectSearch.click();
    await objectSearch.setValue('Ast');
    await browser.waitUntil(async () => (await visibleCount('[data-object-id]')) === 1, {
      timeout: WAIT,
      timeoutMsg: 'Object search did not filter to Aster',
    });
    await capture('search-filters');
    await objectSearch.setValue('');
  });

  it('captures saved Collections, Safe Mode, and a real switch preview', async () => {
    await invokeInApp('toggle_mod_safe', {
      gameId,
      folderPath: unsafeModPath,
      safe: false,
    });
    await invokeInApp<CollectionSummary>('create_collection', {
      gameId,
      name: 'Blue Hour Set',
      saveMode: 'save_current_state',
    });
    await invokeInApp('bulk_toggle_mods', {
      gameId,
      paths: [firstModPath, unsafeModPath],
      enable: false,
      operationId: `landing-collection-${randomUUID()}`,
      intentRevision: null,
      expectedIdentities: null,
    });
    await reconcile(gameId);
    await invokeInApp<CollectionSummary>('create_collection', {
      gameId,
      name: 'After Dark Set',
      saveMode: 'save_current_state',
    });
    await browser.refresh();
    await waitForWorkspaceCoreReady(gameId, WAIT);
    await gotoWorkspaceView('collections');
    await $('//tr[.//span[normalize-space(.)="Blue Hour Set"]]').waitForDisplayed({
      timeout: WAIT,
    });
    await $('//tr[.//span[normalize-space(.)="After Dark Set"]]').waitForDisplayed({
      timeout: WAIT,
    });
    await capture('collections-safe-mode');

    const settings = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', {
      settings: {
        ...settings,
        safety: {
          ...settings.safety,
          runtime_safe_mode_by_game: {
            ...settings.safety.runtime_safe_mode_by_game,
            [gameId]: true,
          },
        },
      },
    });

    const apply = await $(
      '//tr[.//span[normalize-space(.)="Blue Hour Set"]]//button[normalize-space(.)="Apply"]',
    );
    await apply.waitForClickable({ timeout: WAIT });
    await apply.click();
    await $('dialog.modal-open').waitForDisplayed({ timeout: WAIT });
    await browser.waitUntil(
      () => browser.execute(() => !document.querySelector('dialog.modal-open [aria-busy="true"]')),
      { timeout: WAIT, interval: 150, timeoutMsg: 'Collection preview did not settle' },
    );
    await capture('collection-switch-preview');
    await $('//dialog[contains(@class,"modal-open")]//button[normalize-space(.)="Cancel"]').click();
    await $('dialog.modal-open').waitForDisplayed({ reverse: true, timeout: WAIT });
  });

  it('captures the Inbox queue and Smart Classification review', async () => {
    await gotoWorkspaceView('mod-inbox');
    await $('input[aria-label="Select Aster - Ivory Edit"]').waitForDisplayed({ timeout: WAIT });
    await $('input[aria-label="Select After Dark"]').waitForDisplayed({ timeout: WAIT });
    await replaceLocalUserInVisiblePaths(game.fixtureRoot);
    await capture('mod-inbox');

    const review = await $('button*=Review selected');
    await review.waitForClickable({ timeout: WAIT });
    await review.click();
    await $('dialog[aria-labelledby="match-wizard-title"]').waitForDisplayed({ timeout: WAIT });
    await browser.waitUntil(
      () =>
        browser.execute(() => {
          const dialog = document.querySelector('dialog[aria-labelledby="match-wizard-title"]');
          return Boolean(dialog && !dialog.textContent?.includes('Inspecting sources'));
        }),
      { timeout: WAIT, interval: 150, timeoutMsg: 'Import review did not finish analysis' },
    );
    await capture('smart-classification');
  });

  it('captures Play, Storage Optimizer, and game setup', async () => {
    const reviewDialog = await $('dialog[aria-labelledby="match-wizard-title"]');
    await reviewDialog.$('button[aria-label="Close"]').click();
    await reviewDialog.waitForDisplayed({ reverse: true, timeout: WAIT });
    await gotoWorkspaceView('dashboard');
    await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: WAIT });
    await browser.pause(1500);
    await capture('dashboard-play');
    const keyMappingCard = await $('//h2[contains(normalize-space(.), "Active Key Mapping")]/..');
    await keyMappingCard.waitForDisplayed({ timeout: WAIT });
    await browser.waitUntil(async () => (await keyMappingCard.getText()).includes('Gallery Look'), {
      timeout: WAIT,
      interval: 200,
      timeoutMsg: 'Sample key mappings did not load on Home',
    });
    await keyMappingCard.scrollIntoView({ block: 'center' });
    await capture('home-key-mapping');

    await gotoWorkspaceView('storage-optimizer');
    const startScan = await $('//button[normalize-space(.)="Start Scan"]');
    await startScan.waitForClickable({ timeout: WAIT });
    await startScan.click();
    await $('table[aria-label="Duplicate Scan Results"]').waitForDisplayed({ timeout: WAIT });
    await $('//button[@role="tab" and normalize-space(.)="High"]').click();
    await replaceLocalUserInVisiblePaths(game.fixtureRoot);
    await capture('storage-optimizer');

    await gotoWorkspaceView('settings');
    await $('[data-testid="settings-tab-catalog"]').click();
    await $('#catalog-assets-heading').waitForDisplayed({ timeout: WAIT });
    await capture('settings-catalog');
    await $('[data-testid="settings-tab-hotkeys"]').click();
    await $('#hotkeys-settings-heading').waitForDisplayed({ timeout: WAIT });
    await capture('settings-hotkeys');

    await $('[data-testid="settings-tab-games"]').click();
    await $('[data-testid="games-add"]').waitForDisplayed({ timeout: WAIT });
    await $('button[title="Edit Game"]').click();
    await $('dialog[open]').waitForDisplayed({ timeout: WAIT });
    await browser.execute(() => {
      const previewPaths = [
        'D:\\Mod Library\\Studio Library\\Mods',
        'D:\\Mod Library\\Inbox',
        'D:\\Games\\Game.exe',
      ];
      let pathIndex = 0;
      for (const input of document.querySelectorAll<HTMLInputElement>(
        'dialog[open] input[type="text"]',
      )) {
        if (/\\Users\\[^\\]+\\/i.test(input.value)) {
          input.value = previewPaths[pathIndex] ?? 'D:\\Mod Library';
          pathIndex += 1;
        }
      }
    });
    await capture('setup-games');

    await invokeInApp('reset_database');
    await browser.url('http://tauri.localhost/');
    await $('#btn-auto-detect').waitForDisplayed({ timeout: WAIT });
    await capture('setup-xxmi-welcome');
  });
});
