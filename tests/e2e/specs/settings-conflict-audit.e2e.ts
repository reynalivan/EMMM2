import { browser, $, $$, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { BaseDirectory } from '@tauri-apps/api/path';
import {
  addMockMod,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { gotoWorkspaceView, seedGamesAndOpenDashboard } from '../support/app.js';
import { createObject, reconcile, waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';
import type { AppSettings, CustomTheme } from '../../../src/shared/api/tauri/bindings.gen.js';

const IDENTIFIER = 'com.reynalivan.emmm.e2e';
const OUTPUT = path.resolve('.audit-output/audit-next-fixes-20261011');

async function visible(selector: string) {
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
    { timeout: 10_000, timeoutMsg: `Visible element missing: ${selector}` },
  );
  if (!found) throw new Error(`Visible element missing: ${selector}`);
  return found;
}

async function button(text: string) {
  return visible(`//button[normalize-space(.)="${text}"]`);
}

async function openTab(tab: string) {
  await gotoWorkspaceView('settings');
  await (await visible(`[data-testid="settings-tab-${tab}"]`)).click();
}

async function selectGame(name: string, gameId: string) {
  await gotoWorkspaceView('mods');
  await (await visible('button[aria-label="Select Game"]')).click();
  await (await button(name)).click();
  await browser.waitUntil(
    async () => (await invokeInApp<AppSettings>('get_settings')).active_game_id === gameId,
    { timeout: 15_000 },
  );
  await waitForWorkspaceCoreReady(gameId);
}

async function saveTheme(theme: string) {
  const settings = await invokeInApp<AppSettings>('get_settings');
  await invokeInApp('save_settings', { settings: { ...settings, theme } });
  await browser.url('http://tauri.localhost/');
  await $('[data-testid="dashboard-layout"]').waitForExist({ timeout: 25_000 });
  await openTab('general');
}

describe('Settings and conflict audit fixes — native UI/IPC', () => {
  const games: MockGame[] = [];
  let gameA: string;
  let gameB: string;
  let themesDir: string;
  let ownedTheme: { path: string; sha256: string } | null = null;

  before(async () => {
    if ((await invokeInApp<string>('plugin:app|identifier')) !== IDENTIFIER) {
      throw new Error('Settings audit requires the private fixture app');
    }
    games.push(await createMockGame('SettingsAuditA'));
    games.push(await createMockGame('SettingsAuditB'));
    [gameA, gameB] = await seedGamesAndOpenDashboard([
      { game: games[0], name: 'Settings Audit A' },
      { game: games[1], name: 'Settings Audit B' },
    ]);
    await createObject(gameA, 'ConflictAudit', 'Character');
    for (const name of ['KeepFixture', 'OtherFixture']) {
      const folder = await addMockMod(games[0], 'ConflictAudit', name);
      await fs.writeFile(path.join(folder, 'mod.ini'), '[TextureOverrideAudit]\nhash = 1234abcd\n');
    }
    await reconcile(gameA);
    const appData = await invokeInApp<string>('plugin:path|resolve_directory', {
      directory: BaseDirectory.AppData,
    });
    if (path.basename(path.resolve(appData)) !== IDENTIFIER) {
      throw new Error('Theme fixture path is outside the private app profile');
    }
    themesDir = path.join(appData, 'themes');
    await fs.mkdir(OUTPUT, { recursive: true });
  });

  after(async () => {
    for (const game of games) scheduleMockGameRemoval(game);
    if (ownedTheme) {
      // The parent removes this exact owned file after harness process shutdown.
      await fs.writeFile(
        path.join(OUTPUT, 'owned-theme.json'),
        JSON.stringify(ownedTheme, null, 2),
      );
    }
  });

  it('keeps each game homepage isolated after switching through Mods', async () => {
    const urlA = 'https://example.invalid/settings-a';
    const urlB = 'https://example.invalid/settings-b';
    await invokeInApp('browser_set_homepage', { gameId: gameA, url: urlA });
    await invokeInApp('browser_set_homepage', { gameId: gameB, url: urlB });
    await openTab('browser');
    const input = await visible('input[aria-label="Homepage URL"]');
    await expect(input).toHaveValue(urlA);
    await selectGame('Settings Audit B', gameB);
    await openTab('browser');
    const inputB = await visible('input[aria-label="Homepage URL"]');
    await (await visible('#browser-storage-heading')).click();
    await expect(inputB).toHaveValue(urlB);
    expect(await invokeInApp<string>('browser_get_homepage', { gameId: gameA })).toBe(urlA);
    expect(await invokeInApp<string>('browser_get_homepage', { gameId: gameB })).toBe(urlB);
    const updatedB = 'https://example.invalid/settings-b-edited';
    await inputB.setValue(updatedB);
    await (await visible('#browser-storage-heading')).click();
    await browser.waitUntil(
      async () =>
        (await invokeInApp<string>('browser_get_homepage', { gameId: gameB })) === updatedB,
      { timeout: 10_000 },
    );
    expect(await invokeInApp<string>('browser_get_homepage', { gameId: gameA })).toBe(urlA);
  });

  it('retains the unsaved AI endpoint after an actual enabled-only save', async () => {
    await openTab('ai');
    const input = await visible('input[aria-label="Endpoint"]');
    const draft = 'https://example.invalid/unsaved-endpoint';
    await input.setValue(draft);
    const before = await invokeInApp<AppSettings>('get_settings');
    await (await visible('input[aria-label="Enable reranking"]')).click();
    await browser.waitUntil(
      async () => (await invokeInApp<AppSettings>('get_settings')).ai.enabled !== before.ai.enabled,
      { timeout: 10_000 },
    );
    await expect(input).toHaveValue(draft);
    expect((await invokeInApp<AppSettings>('get_settings')).ai.base_url).toBe(before.ai.base_url);
    await (await visible('input[aria-label="Enable reranking"]')).click();
    await browser.waitUntil(
      async () => (await invokeInApp<AppSettings>('get_settings')).ai.enabled === before.ai.enabled,
      { timeout: 10_000 },
    );
  });

  it('uses Onyx for legacy dark when no custom dark theme exists', async () => {
    const themes = await invokeInApp<{ id: string }[]>('list_custom_themes');
    if (themes.some((theme) => theme.id === 'dark'))
      throw new Error('Legacy fixture requires no existing private custom dark theme');
    await saveTheme('dark');
    await expect(await $('#theme-select')).toHaveValue('onyx');
    await browser.waitUntil(async () => (await $('html').getAttribute('data-theme')) === 'onyx', {
      timeout: 10_000,
    });
    expect((await invokeInApp<AppSettings>('get_settings')).theme).toBe('dark');
  });

  it('preserves and applies a valid custom dark theme through native loading', async () => {
    const theme: CustomTheme = {
      id: 'dark',
      label: 'Owned Custom Dark',
      config: {
        colors: { primary: '#123456' },
        glass: {},
        liquid: {},
        background: { kind: 'solid', value: '#101010', dim_opacity: 0.2 },
      },
    };
    await fs.mkdir(themesDir, { recursive: true });
    const bytes = JSON.stringify(theme);
    const target = path.join(themesDir, 'dark.json');
    await fs.writeFile(target, bytes, { flag: 'wx' });
    ownedTheme = { path: target, sha256: createHash('sha256').update(bytes).digest('hex') };
    expect((await invokeInApp<CustomTheme>('load_custom_theme', { id: 'dark' })).label).toBe(
      theme.label,
    );
    await saveTheme('dark');
    await expect(await $('#theme-select')).toHaveValue('dark');
    await browser.waitUntil(async () => (await $('html').getAttribute('data-theme')) === 'dark', {
      timeout: 10_000,
    });
    const readPrimary = () =>
      browser.execute(() =>
        getComputedStyle(document.documentElement).getPropertyValue('--color-primary').trim(),
      );
    await browser.waitUntil(async () => (await readPrimary()) === '#123456', {
      timeout: 10_000,
      timeoutMsg: 'Native custom theme variables did not apply',
    });
    const primary = await readPrimary();
    expect(primary).toBe('#123456');
  });

  it('closes an idle Conflict dialog with Escape and opens it again', async () => {
    await selectGame('Settings Audit A', gameA);
    await (await visible('button[aria-label^="Shared hashes:"]')).click();
    const dialog = await visible(
      '//dialog[@open][.//h3[normalize-space(.)="Enabled Mod Conflicts"]]',
    );
    await expect(dialog).toBeDisplayed();
    await browser.keys('Escape');
    await dialog.waitForDisplayed({ reverse: true, timeout: 5_000 });
    await (await visible('button[aria-label^="Shared hashes:"]')).click();
    await expect(
      await visible('//dialog[@open][.//h3[normalize-space(.)="Enabled Mod Conflicts"]]'),
    ).toBeDisplayed();
    await browser.keys('Escape');
  });

  it('rejects builtin custom IDs and removes the unused theme IPC wrappers', async () => {
    expect((await invokeInApp<CustomTheme>('load_custom_theme', { id: 'dark' })).id).toBe('dark');
    for (const id of ['onyx', 'light', 'system']) {
      await expect(invokeInApp('load_custom_theme', { id })).rejects.toThrow(/reserved|built.?in/i);
    }
    for (const command of ['save_custom_theme', 'delete_custom_theme', 'export_custom_theme']) {
      await expect(invokeInApp(command)).rejects.toThrow(/not found|not allowed/i);
    }
  });
});
