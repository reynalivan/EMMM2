import { browser, $, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import type { AppSettings, CatalogPackStatus } from '../../../src/shared/api/tauri/bindings.gen.js';
import {
  addMockMod,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { gotoWorkspaceView, seedGamesAndOpenDashboard } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import { pathsEqual } from '../../../src/shared/lib/pathKey.js';

const E2E_IDENTIFIER = 'com.reynalivan.emmm.e2e';
const APP_URL = 'http://tauri.localhost/';
const SETTINGS = '[data-testid="settings-page"]';
const OPEN_DIALOG = 'dialog[open]';
const QA_GAME_NAME = 'QA Settings Added Game';
const QA_RENAMED_GAME = 'QA Settings Renamed Game';
const DUMMY_QA_KEY = 'qa-not-a-real-api-key-settings-smoke';
const EVIDENCE_DIR = path.resolve('logs/client-smoke-20261007/settings');
const RUN_STAMP = new Date().toISOString().replace(/[:.]/g, '-');
const TABS = [
  ['general', 'appearance-heading'],
  ['games', 'games-settings-heading'],
  ['catalog', 'catalog-assets-heading'],
  ['browser', 'browser-settings-heading'],
  ['privacy', 'privacy-settings-heading'],
  ['hotkeys', 'hotkeys-settings-heading'],
  ['ai', 'ai-settings-heading'],
  ['maintenance', 'system-maintenance-heading'],
  ['integrations', 'integrations-heading'],
  ['logs', 'logs-heading'],
] as const;
type SettingsTab = (typeof TABS)[number][0];

export const UNEXERCISED_SETTINGS_CONTROLS = [
  'Native file/folder pickers, theme import/export, support and release links',
  'Application/catalog network checks, catalog install, AI Test Connection',
  'Enabling persisted OS-global hotkeys, sending keys to a real game',
  'Enabling telemetry, configuring external executables or real credentials',
  'Game source-library migration and real launcher/game execution',
  'Positive application reset: restores globally enabled hotkey defaults',
  'Installed catalog refresh success requires a locally installed fixture catalog pack',
] as const;

interface UiProbeEvent {
  event: string;
  label: string;
  name: string | null;
  trusted: boolean;
  valueLength: number | null;
  selectedValue: string | null;
  checked: boolean | null;
}

interface UiProbe {
  events: UiProbeEvent[];
  messages: string[];
  bridge: { invokeWritable: boolean | null; invokeConfigurable: boolean | null };
}

interface ProbeWindow extends Window {
  __EMMM_QA_SETTINGS_PROBE__?: UiProbe;
  __TAURI_INTERNALS__?: object;
}

function scalarSettings(settings: AppSettings) {
  return {
    revision: settings.revision,
    theme: settings.theme,
    language: settings.language,
    autoCloseLauncher: settings.auto_close_launcher,
    keywords: settings.safety.keywords,
    hotkeys: settings.hotkeys,
    keyviewer: settings.keyviewer,
    ai: {
      enabled: settings.ai.enabled,
      hasApiKey: settings.ai.has_api_key,
      endpoint:
        settings.ai.base_url === 'https://example.invalid/emmm-qa'
          ? 'qa-fixture'
          : settings.ai.base_url
            ? 'configured'
            : 'empty',
    },
  };
}

let currentFailure: string | undefined;
let nativeBefore: ReturnType<typeof scalarSettings> | undefined;

function qaCase(title: string, action: () => Promise<void>): void {
  it(title, async () => {
    try {
      await action();
    } catch (error: unknown) {
      currentFailure = (
        error instanceof Error ? (error.stack ?? error.message) : String(error)
      ).replaceAll(DUMMY_QA_KEY, '[REDACTED]');
      throw error;
    }
  });
}

async function installUiProbe(): Promise<void> {
  await browser.execute((dummyKey: string) => {
    const target = window as ProbeWindow;
    const internals = target.__TAURI_INTERNALS__;
    const descriptor = internals ? Object.getOwnPropertyDescriptor(internals, 'invoke') : undefined;
    const probe: UiProbe = {
      events: [],
      messages: [],
      bridge: {
        invokeWritable: descriptor?.writable ?? null,
        invokeConfigurable: descriptor?.configurable ?? null,
      },
    };
    target.__EMMM_QA_SETTINGS_PROBE__ = probe;
    for (const eventName of ['input', 'change', 'click', 'blur', 'keydown']) {
      document.addEventListener(
        eventName,
        (event: Event) => {
          const target = event.target;
          if (!(target instanceof HTMLElement)) return;
          const input = target instanceof HTMLInputElement ? target : null;
          const select = target instanceof HTMLSelectElement ? target : null;
          const sensitive =
            input?.type === 'password' || input?.getAttribute('aria-label') === 'API key';
          probe.events.push({
            event: event.type,
            label: (
              target.getAttribute('aria-label') ??
              target.getAttribute('title') ??
              target.textContent?.trim() ??
              ''
            ).slice(0, 120),
            name: target.getAttribute('name'),
            trusted: event.isTrusted,
            valueLength: input && !sensitive ? input.value.length : null,
            selectedValue: select?.value ?? null,
            checked: input?.type === 'checkbox' ? input.checked : null,
          });
          if (probe.events.length > 500) probe.events.shift();
        },
        true,
      );
    }
    new MutationObserver(() => {
      for (const element of document.querySelectorAll('[role="alert"],[role="status"]')) {
        const text = element.textContent?.trim().replaceAll(dummyKey, '[REDACTED]').slice(0, 700);
        if (text && !probe.messages.includes(text)) probe.messages.push(text);
      }
    }).observe(document.body, { childList: true, subtree: true, characterData: true });
  }, DUMMY_QA_KEY);
}

async function readUiProbe(): Promise<UiProbe | null> {
  return browser.execute(() => (window as ProbeWindow).__EMMM_QA_SETTINGS_PROBE__ ?? null);
}

async function click(selector: string): Promise<void> {
  const element = await $(selector);
  await element.waitForDisplayed({ timeout: 10_000 });
  await element.scrollIntoView({ block: 'center' });
  await element.waitForClickable({ timeout: 10_000 });
  await element.click();
}

async function clickButton(text: string, scope = SETTINGS): Promise<void> {
  const button = await findButton(text, scope);
  await button.waitForDisplayed({ timeout: 10_000 });
  await button.scrollIntoView({ block: 'center' });
  await button.waitForClickable({ timeout: 10_000 });
  await button.click();
}

async function findButton(text: string, scope = SETTINGS): Promise<WebdriverIO.Element> {
  return (await $(scope)).$(`button=${text}`).getElement();
}

async function fill(selector: string, value: string): Promise<void> {
  const input = await $(selector);
  await input.waitForDisplayed({ timeout: 10_000 });
  await input.scrollIntoView({ block: 'center' });
  await input.waitForEnabled({ timeout: 10_000 });
  await input.click();
  await browser.keys(['Control', 'a']);
  await browser.keys('Backspace');
  if (value) await browser.keys(Array.from(value));
}

async function select(selector: string, value: string): Promise<void> {
  const control = await $(selector);
  await control.waitForEnabled({ timeout: 10_000 });
  await control.scrollIntoView({ block: 'center' });
  const options = await control.$$('option').getElements();
  let selectedIndex = -1;
  for (let index = 0; index < options.length; index += 1) {
    if ((await options[index].getAttribute('value')) === value) selectedIndex = index;
  }
  if (selectedIndex < 0) throw new Error(`Select has no visible option with value ${value}`);
  await control.click();
  await browser.keys('Home');
  for (let index = 0; index < selectedIndex; index += 1) await browser.keys('ArrowDown');
  await browser.keys('Enter');
  await browser.keys('Tab');
  await expect(control).toHaveValue(value);
}

async function openTab(tab: SettingsTab): Promise<void> {
  await click(`[data-testid="settings-tab-${tab}"]`);
  await expect(await $(`[data-testid="settings-tab-${tab}"]`)).toHaveAttribute(
    'aria-current',
    'page',
  );
  const heading = TABS.find(([id]) => id === tab)?.[1];
  if (!heading) throw new Error(`Unknown settings section: ${tab}`);
  await $(`#${heading}`).waitForDisplayed({ timeout: 10_000 });
}

async function readSettings(): Promise<AppSettings> {
  return invokeInApp<AppSettings>('get_settings');
}

async function expectSettings(
  message: string,
  predicate: (settings: AppSettings) => boolean,
): Promise<void> {
  let observed: ReturnType<typeof scalarSettings> | null = null;
  try {
    await browser.waitUntil(
      async () => {
        const settings = await readSettings();
        observed = scalarSettings(settings);
        return predicate(settings);
      },
      {
        timeout: 10_000,
        interval: 100,
        timeoutMsg: message,
      },
    );
  } catch (error: unknown) {
    throw new Error(`${message}; native observed=${JSON.stringify(observed)}`, { cause: error });
  }
}

async function assertIsolatedApplication(): Promise<void> {
  expect(await invokeInApp<string>('plugin:app|identifier')).toBe(E2E_IDENTIFIER);
}

async function expectToast(text: string): Promise<void> {
  await $(`//*[(@role='alert' or @role='status') and contains(., '${text}')]`).waitForDisplayed({
    timeout: 10_000,
  });
}

async function fillGame(name: string, modsPath: string, exePath: string): Promise<void> {
  await fill(`${OPEN_DIALOG} input[placeholder="e.g. GIMI"]`, name);
  await fill(`${OPEN_DIALOG} input[placeholder="C:/Games/GIMI/Mods"]`, modsPath);
  await fill(`${OPEN_DIALOG} input[placeholder="C:/Games/GIMI/Game.exe"]`, exePath);
}

async function gameAction(name: string, title: string): Promise<void> {
  const button = await $(
    `//section[@aria-labelledby='games-settings-heading']//h3[contains(., '${name}')]/ancestor::div[contains(@class,'sm:flex-row')][1]//button[@title='${title}']`,
  );
  await button.waitForDisplayed({ timeout: 10_000 });
  await button.scrollIntoView({ block: 'center' });
  await button.waitForClickable({ timeout: 10_000 });
  await button.click();
}

describe('Client settings smoke — isolated native UI controls', () => {
  const fixtures: MockGame[] = [];
  let baseline: AppSettings;
  let baseGameId: string;
  let sentinelPath: string;
  let initialHomepage: string;
  let initialRetention: number;
  let dummyKeyAttempted = false;
  let evidenceIndex = 0;

  before(async () => {
    await assertIsolatedApplication();
    fixtures.push(await createMockGame('QA_Settings_Base'));
    fixtures.push(await createMockGame('QA_Settings_Added'));
    const modPath = await addMockMod(fixtures[0], 'QA Settings Object', 'QA Settings Mod');
    sentinelPath = path.join(modPath, 'mod.ini');
    [baseGameId] = await seedGamesAndOpenDashboard([
      { game: fixtures[0], name: 'QA Settings Base Game' },
    ]);
    baseline = await readSettings();
    expect(baseline.hotkeys?.enabled).toBe(false);
    expect(baseline.ai.has_api_key).toBe(false);
    initialHomepage = await invokeInApp<string>('browser_get_homepage', { gameId: baseGameId });
    initialRetention = await invokeInApp<number>('browser_get_retention_days', {
      legacyRetentionDays: null,
    });
    console.info(`EMMM_QA_UNEXERCISED ${JSON.stringify(UNEXERCISED_SETTINGS_CONTROLS)}`);
  });

  beforeEach(async () => {
    currentFailure = undefined;
    await assertIsolatedApplication();
    const current = await readSettings();
    if (!baseline.hotkeys) throw new Error('QA baseline omitted hotkeys');
    await invokeInApp('save_settings', {
      settings: {
        ...baseline,
        revision: current.revision,
        language: 'en',
        theme: 'onyx',
        hotkeys: { ...baseline.hotkeys, enabled: false },
        keyviewer: { ...baseline.keyviewer, enabled: false },
        ai: { ...baseline.ai, enabled: false },
        diagnostics: { ...baseline.diagnostics, telemetry_enabled: false },
      },
    });
    await invokeInApp('browser_set_homepage', { gameId: baseGameId, url: initialHomepage });
    await invokeInApp('browser_set_retention_days', { days: initialRetention });
    await browser.url(APP_URL);
    await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: 25_000 });
    await gotoWorkspaceView('settings');
    await $(SETTINGS).waitForDisplayed({ timeout: 10_000 });
    nativeBefore = scalarSettings(await readSettings());
    await installUiProbe();
  });

  afterEach(async function () {
    if (this.currentTest?.state === 'failed' && fixtures[0]) {
      evidenceIndex += 1;
      await fs.mkdir(EVIDENCE_DIR, { recursive: true });
      const stem = `${RUN_STAMP}-${String(evidenceIndex).padStart(2, '0')}-${this.currentTest.title.replace(/[^a-zA-Z0-9_-]/g, '_')}`;
      const nativeAfter = scalarSettings(await readSettings());
      const uiProbe = await readUiProbe();
      const originalVisibility = await browser.execute(() =>
        Array.from(
          document.querySelectorAll<HTMLInputElement>(
            'input[type="password"],input[aria-label="API key"]',
          ),
        ).map((input) => {
          const previous = input.style.visibility;
          input.style.visibility = 'hidden';
          return previous;
        }),
      );
      const captures = await Promise.allSettled([
        browser.saveScreenshot(path.join(EVIDENCE_DIR, `${stem}.png`)),
        browser
          .execute(() => ({
            url: location.href,
            text: document.body.innerText.slice(0, 18_000),
            controls: Array.from(
              document.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLButtonElement>(
                'input,select,button',
              ),
            ).map((element) => ({
              tag: element.tagName,
              label:
                element.getAttribute('aria-label') ??
                element.getAttribute('title') ??
                element.textContent?.trim(),
              value:
                element instanceof HTMLInputElement &&
                (element.type === 'password' || element.getAttribute('aria-label') === 'API key')
                  ? '[REDACTED]'
                  : element.value,
              disabled: element.disabled,
            })),
          }))
          .then((dom) =>
            fs.writeFile(
              path.join(EVIDENCE_DIR, `${stem}.json`),
              JSON.stringify(
                {
                  test: this.currentTest?.fullTitle(),
                  error:
                    currentFailure ??
                    this.currentTest?.err?.message.replaceAll(DUMMY_QA_KEY, '[REDACTED]'),
                  nativeBefore,
                  nativeAfter,
                  uiProbe,
                  dom,
                },
                null,
                2,
              ),
            ),
          ),
      ]);
      await browser.execute((styles: string[]) => {
        document
          .querySelectorAll<HTMLInputElement>('input[type="password"],input[aria-label="API key"]')
          .forEach((input, index) => {
            input.style.visibility = styles[index] ?? '';
          });
      }, originalVisibility);
      console.error(`EMMM_QA_FAILURE_ARTIFACT ${path.join(EVIDENCE_DIR, stem)}`);
      for (const capture of captures) {
        if (capture.status === 'rejected') {
          console.error('Could not capture QA failure evidence:', capture.reason);
        }
      }
    }
    const probe = await readUiProbe();
    console.info(
      `EMMM_QA_SETTINGS_SCALARS ${JSON.stringify({
        test: this.currentTest?.title,
        before: nativeBefore,
        after: scalarSettings(await readSettings()),
        events: {
          input: probe?.events.filter((event) => event.event === 'input').length,
          change: probe?.events.filter((event) => event.event === 'change').length,
          trusted: probe?.events.filter((event) => event.trusted).length,
          latestChanges: probe?.events.filter((event) => event.event === 'change').slice(-4),
        },
        messages: probe?.messages.slice(-4),
        bridge: probe?.bridge,
      })}`,
    );
    if (dummyKeyAttempted) {
      await assertIsolatedApplication();
      await invokeInApp('delete_ai_api_key');
      dummyKeyAttempted = false;
    }
    expect((await readSettings()).hotkeys?.enabled).toBe(false);
  });

  after(() => {
    for (const fixture of fixtures) scheduleMockGameRemoval(fixture);
  });

  for (const [tab, heading] of TABS) {
    qaCase(`opens the ${tab} Settings tab through its visible navigation button`, async () => {
      await openTab(tab);
      await expect(await $(`#${heading}`)).toBeDisplayed();
      await expect(await $(SETTINGS)).not.toHaveText(/Something went wrong/);
    });
  }

  qaCase(
    'cold startup can persist its first Settings save without external writes after rendering',
    async () => {
      await openTab('privacy');
      const keyword = 'qa-cold-start-revision';
      await fill('input[placeholder="e.g. skin, lewd, bikini"]', keyword);
      await clickButton('Add Keyword');
      await expectSettings('First cold-start Settings save did not persist', (settings) =>
        settings.safety.keywords.includes(keyword),
      );
    },
  );

  qaCase(
    'cache-refreshed Settings save persists after Close after launch publishes the current snapshot',
    async () => {
      await openTab('general');
      const initial = (await readSettings()).auto_close_launcher;
      await click('input[aria-label="Close after launch"]');
      await expectSettings(
        'Close after launch did not update its own field',
        (settings) => settings.auto_close_launcher === !initial,
      );
      await click('input[aria-label="Close after launch"]');
      await expectSettings(
        'Close after launch did not restore its own field',
        (settings) => settings.auto_close_launcher === initial,
      );
      await openTab('privacy');
      const keyword = 'qa-cache-refreshed-revision';
      await fill('input[placeholder="e.g. skin, lewd, bikini"]', keyword);
      await clickButton('Add Keyword');
      await expectSettings('Settings save still rejected after UI cache refresh', (settings) =>
        settings.safety.keywords.includes(keyword),
      );
      await click(`button[aria-label="Remove ${keyword}"]`);
      await expectSettings(
        'Refreshed Settings removal did not persist',
        (settings) => !settings.safety.keywords.includes(keyword),
      );
    },
  );

  for (const theme of ['system', 'onyx', 'light'] as const) {
    qaCase(`persists the ${theme} theme selected in General`, async () => {
      await openTab('general');
      const primeTheme = theme === 'light' ? 'onyx' : 'light';
      if ((await readSettings()).theme !== primeTheme) {
        await select('select[aria-label="Theme"]', primeTheme);
        await expectSettings(
          `Priming ${primeTheme} theme did not persist`,
          (settings) => settings.theme === primeTheme,
        );
      }
      await select('select[aria-label="Theme"]', theme);
      await expectSettings(
        `Theme ${theme} did not persist`,
        (settings) => settings.theme === theme,
      );
      await openTab('games');
      await openTab('general');
      await expect(await $('select[aria-label="Theme"]')).toHaveValue(theme);
    });
  }

  for (const language of ['id', 'zh'] as const) {
    qaCase(
      `changes the interface to ${language} and restores English through the select`,
      async () => {
        await openTab('general');
        const languageSelect =
          'section[aria-labelledby="appearance-heading"] select:has(option[value="en"]):has(option[value="zh"])';
        try {
          await select(languageSelect, language);
          await expectSettings(
            `Language ${language} did not persist`,
            (settings) => settings.language === language,
          );
          await expect(await $(languageSelect)).toHaveValue(language);
          await expect(await $('[data-testid="settings-tab-general"]')).not.toHaveText('General');
        } finally {
          await select(languageSelect, 'en');
          await expectSettings(
            'English restoration did not persist',
            (settings) => settings.language === 'en',
          );
        }
        await expect(await $('[data-testid="settings-tab-general"]')).toHaveText('General');
      },
    );
  }

  qaCase(
    'round-trips Close after launch through its checkbox without launching anything',
    async () => {
      await openTab('general');
      const initial = (await readSettings()).auto_close_launcher;
      const toggle = 'input[aria-label="Close after launch"]';
      expect(await $(toggle).isSelected()).toBe(initial);
      await click(toggle);
      await expectSettings(
        'Close after launch did not persist',
        (settings) => settings.auto_close_launcher === !initial,
      );
      expect(await $(toggle).isSelected()).toBe(!initial);
      await click(toggle);
      await expectSettings(
        'Close after launch did not restore',
        (settings) => settings.auto_close_launcher === initial,
      );
      expect(await $(toggle).isSelected()).toBe(initial);
    },
  );

  for (const title of ['Privacy Policy', 'Terms of Use'] as const) {
    qaCase(
      `opens ${title}, closes with its button, then closes a fresh dialog with Escape`,
      async () => {
        await openTab('general');
        const card = `//section[@aria-labelledby='system-heading']//button[.//span[normalize-space(.)='${title}']]`;
        await click(card);
        await expect(await $('#trust-information-title')).toHaveText(title);
        await expect(await $(`${OPEN_DIALOG} h4`)).toBeDisplayed();
        await click('button[aria-label="Close dialog"]');
        await expect(await $('#trust-information-title')).not.toExist();
        await click(card);
        await expect(await $('#trust-information-title')).toHaveText(title);
        await browser.keys('Escape');
        await expect(await $('#trust-information-title')).not.toExist();
      },
    );
  }

  qaCase('ignores empty Safety keywords without changing persisted classifications', async () => {
    await openTab('privacy');
    const before = (await readSettings()).safety.keywords;
    await fill('input[placeholder="e.g. skin, lewd, bikini"]', '   ');
    await clickButton('Add Keyword');
    expect((await readSettings()).safety.keywords).toEqual(before);
  });

  qaCase(
    'adds a Unicode Safety keyword, warns on normalized duplicate, then removes it',
    async () => {
      await openTab('privacy');
      const keyword = 'qa-安全-é';
      await fill('input[placeholder="e.g. skin, lewd, bikini"]', `  ${keyword}  `);
      await clickButton('Add Keyword');
      await expectSettings('Unicode keyword did not persist', (settings) =>
        settings.safety.keywords.includes(keyword),
      );
      await expect(await $(`button[aria-label="Remove ${keyword}"]`)).toBeDisplayed();
      await fill('input[placeholder="e.g. skin, lewd, bikini"]', ` ${keyword.toUpperCase()} `);
      await clickButton('Add Keyword');
      await expectToast('Keyword already exists');
      expect(
        (await readSettings()).safety.keywords.filter((value) => value === keyword),
      ).toHaveLength(1);
      await click(`button[aria-label="Remove ${keyword}"]`);
      await expectSettings(
        'Removed keyword remained persisted',
        (settings) => !settings.safety.keywords.includes(keyword),
      );
      await expect(await $(`button[aria-label="Remove ${keyword}"]`)).not.toExist();
    },
  );

  qaCase('keeps global hotkeys disabled while editing, resetting, and saving drafts', async () => {
    await openTab('hotkeys');
    const globalToggle =
      'section[aria-labelledby="hotkeys-settings-heading"] input[type="checkbox"]';
    const overlayToggle =
      'section[aria-labelledby="keyviewer-settings-heading"] input[type="checkbox"]';
    await expect(await $(globalToggle)).not.toBeSelected();
    await expect(await $('input[aria-label="Safe Mode"]')).toBeDisabled();
    await click(globalToggle);
    for (const [label, value] of [
      ['Safe Mode', 'F9'],
      ['Previous preset', 'Shift+F9'],
      ['Next preset', 'Ctrl+F9'],
    ] as const) {
      await fill(`input[aria-label="${label}"]`, value);
    }
    await click(overlayToggle);
    await fill('input[aria-label="Toggle overlay"]', 'F11');
    await expect(await $('button[title="Reset to default"]')).toBeDisplayed();
    await click('input[aria-label="Toggle overlay"] + button[title="Reset to default"]');
    await expect(await $('input[aria-label="Toggle overlay"]')).toHaveValue('F7');
    await clickButton('Reset All to Defaults');
    await expect(await $('input[aria-label="Safe Mode"]')).toHaveValue('F5');
    await expect(await $('input[aria-label="Previous preset"]')).toHaveValue('Shift+F5');
    await expect(await $('input[aria-label="Next preset"]')).toHaveValue('Ctrl+F5');
    await click(globalToggle);
    await click(overlayToggle);
    await click('#preset-status-overlay-enabled');
    await expect(await $(globalToggle)).not.toBeSelected();
    await expect(await $(overlayToggle)).not.toBeSelected();
    await expect(await $('#preset-status-overlay-enabled')).toBeSelected();
    await clickButton('Save controls');
    await expectSettings(
      'Safe draft save did not persist',
      (settings) =>
        settings.hotkeys?.enabled === false &&
        settings.hotkeys.preset_status_overlay_enabled === true,
    );
    const toast = await $('[role="status"]');
    await toast.waitForDisplayed({ timeout: 10_000 });
    const toastText = await toast.getText();
    console.info(`EMMM_QA_HOTKEY_SAVE_TOAST ${toastText}`);
    expect(toastText).not.toMatch(/hotkeys\.save_success/);
    await expect(await $(globalToggle)).not.toBeSelected();
    await expect(await $(overlayToggle)).not.toBeSelected();
  });

  for (const [binding, message] of [
    ['F5', 'share key'],
    ['F6', 'package toggle'],
    ['F8', 'frame analysis'],
  ] as const) {
    qaCase(
      `blocks saving an overlay binding conflicting with ${binding} while OS globals remain off`,
      async () => {
        await openTab('hotkeys');
        await click('section[aria-labelledby="keyviewer-settings-heading"] input[type="checkbox"]');
        await fill('input[aria-label="Toggle overlay"]', binding);
        await expect(await $(`${SETTINGS} [role="alert"]`)).toHaveText(new RegExp(message));
        await expect(await findButton('Save controls')).toBeDisabled();
        expect((await readSettings()).hotkeys?.enabled).toBe(false);
        expect((await readSettings()).hotkeys?.toggle_overlay).toBe(
          baseline.hotkeys?.toggle_overlay,
        );
      },
    );
  }

  qaCase(
    'keeps an incomplete Game form invalid and cancels without creating a configuration',
    async () => {
      await openTab('games');
      const before = (await readSettings()).games.length;
      await click('[data-testid="games-add"]');
      await expect(await $('[data-testid="game-form-submit"]')).toBeDisabled();
      await fillGame(
        'QA Invalid Game',
        path.join(fixtures[1].modsPath, 'bad*path'),
        fixtures[1].exePath,
      );
      await expect(await $(OPEN_DIALOG)).toHaveText(/Path contains invalid characters/);
      await expect(await $('[data-testid="game-form-submit"]')).toBeDisabled();
      await clickButton('Back', OPEN_DIALOG);
      expect((await readSettings()).games).toHaveLength(before);
      await expect(await $(OPEN_DIALOG)).not.toExist();
    },
  );

  qaCase('rejects an already registered Mods path in the Add Game form', async () => {
    await openTab('games');
    const configured = (await readSettings()).games.find((game) => game.id === baseGameId);
    if (!configured) throw new Error('Owned baseline game is missing before duplicate validation');
    const verbatimPrefix = '\\\\?\\';
    const configuredVerbatim = configured.mod_path.startsWith(verbatimPrefix);
    console.info(
      `EMMM_QA_DUPLICATE_PATH_FACTS ${JSON.stringify({
        configuredVerbatim,
        candidateVerbatim: fixtures[0].modsPath.startsWith(verbatimPrefix),
        frontendPathsEqual: pathsEqual(configured.mod_path, fixtures[0].modsPath),
        equalAfterVerbatimPrefixRemoval: pathsEqual(
          configuredVerbatim ? configured.mod_path.slice(4) : configured.mod_path,
          fixtures[0].modsPath,
        ),
        candidateHasInvalidFormCharacters: /[?*<>|]/.test(fixtures[0].modsPath),
      })}`,
    );
    await click('[data-testid="games-add"]');
    await fillGame('QA Duplicate Game', fixtures[0].modsPath, fixtures[0].exePath);
    await expect(await $(OPEN_DIALOG)).toHaveText(/Already registered/);
    await expect(await $('[data-testid="game-form-submit"]')).toBeDisabled();
    await clickButton('Back', OPEN_DIALOG);
    expect((await readSettings()).games.some((game) => game.name === 'QA Duplicate Game')).toBe(
      false,
    );
  });

  qaCase(
    'adds, edits, cancels removal, and removes only an owned temporary Game configuration',
    async () => {
      await openTab('games');
      await click('[data-testid="games-add"]');
      await fillGame(QA_GAME_NAME, fixtures[1].modsPath, fixtures[1].exePath);
      await fill(
        '//dialog[@open]//label[.//span[normalize-space(.)="Mod Inbox (Optional)"]]/following-sibling::div//input',
        path.join(fixtures[1].root, 'incoming'),
      );
      await click('[data-testid="game-form-submit"]');
      await expectSettings('Added QA Game did not persist', (settings) =>
        settings.games.some((game) => game.name === QA_GAME_NAME),
      );
      await gameAction(QA_GAME_NAME, 'Edit Game');
      await fill(`${OPEN_DIALOG} input[placeholder="e.g. GIMI"]`, QA_RENAMED_GAME);
      await click('[data-testid="game-form-submit"]');
      await expectSettings('Edited QA Game name did not persist', (settings) =>
        settings.games.some((game) => game.name === QA_RENAMED_GAME),
      );
      await gameAction(QA_RENAMED_GAME, 'Remove Game');
      await clickButton('Cancel', OPEN_DIALOG);
      expect((await readSettings()).games.some((game) => game.name === QA_RENAMED_GAME)).toBe(true);
      await gameAction(QA_RENAMED_GAME, 'Remove Game');
      await clickButton('Confirm', OPEN_DIALOG);
      await expectSettings(
        'Removed QA Game remained registered',
        (settings) => !settings.games.some((game) => game.name === QA_RENAMED_GAME),
      );
      expect((await readSettings()).games.some((game) => game.id === baseGameId)).toBe(true);
      expect(await fs.readFile(path.join(fixtures[1].root, 'd3dx.ini'), 'utf8')).toBe('[Main]\n');
    },
  );

  qaCase(
    'round-trips and resets the Browser homepage using blur without loading a website',
    async () => {
      await openTab('browser');
      const homepage = 'input[aria-label="Homepage URL"]';
      const qaUrl = 'https://example.invalid/emmm-qa';
      await fill(homepage, qaUrl);
      await click('#browser-storage-heading');
      await browser.waitUntil(
        async () =>
          (await invokeInApp<string>('browser_get_homepage', { gameId: baseGameId })) === qaUrl,
        { timeout: 10_000, timeoutMsg: 'Homepage blur save did not persist' },
      );
      await openTab('general');
      await openTab('browser');
      await expect(await $(homepage)).toHaveValue(qaUrl);
      await clickButton('Reset', 'section[aria-labelledby="browser-settings-heading"]');
      await expect(await $(homepage)).toHaveValue('https://www.google.com');
      await browser.waitUntil(
        async () =>
          (await invokeInApp<string>('browser_get_homepage', { gameId: baseGameId })) ===
          'https://www.google.com',
        { timeout: 10_000 },
      );
    },
  );

  qaCase('rejects a non-HTTP Browser homepage without persisting or navigating to it', async () => {
    await openTab('browser');
    await fill('input[aria-label="Homepage URL"]', 'file:///emmm-qa-not-a-real-file');
    await click('#browser-storage-heading');
    await expectToast('Failed to update homepage:');
    expect(await invokeInApp<string>('browser_get_homepage', { gameId: baseGameId })).toBe(
      initialHomepage,
    );
    expect(await browser.getUrl()).toContain('tauri.localhost');
  });

  for (const days of [1, 365] as const) {
    qaCase(`saves Browser retention boundary ${days} through blur`, async () => {
      await openTab('browser');
      await fill('input[aria-label="Keep archives"]', String(days));
      await click('#browser-settings-heading');
      await browser.waitUntil(
        async () =>
          (await invokeInApp<number>('browser_get_retention_days', {
            legacyRetentionDays: null,
          })) === days,
        { timeout: 10_000, timeoutMsg: `Retention ${days} did not persist` },
      );
      await openTab('general');
      await openTab('browser');
      await expect(await $('input[aria-label="Keep archives"]')).toHaveValue(String(days));
    });
  }

  for (const invalidDays of ['0', '366', '1.5', ''] as const) {
    qaCase(
      `rejects Browser retention ${invalidDays || 'empty'} and restores the saved value`,
      async () => {
        await openTab('browser');
        const retention = 'input[aria-label="Keep archives"]';
        await expect(await $(retention)).toHaveValue(String(initialRetention));
        await fill(retention, invalidDays);
        await click('#browser-settings-heading');
        await expectToast('Retention days must be a whole number from 1 to 365.');
        await expect(await $(retention)).toHaveValue(String(initialRetention));
        expect(
          await invokeInApp<number>('browser_get_retention_days', { legacyRetentionDays: null }),
        ).toBe(initialRetention);
      },
    );
  }

  qaCase(
    'shows/hides a dummy AI draft, saves a blank key, then saves and removes only the QA credential',
    async () => {
      await assertIsolatedApplication();
      await openTab('ai');
      await expect(await $('input[aria-label="Enable reranking"]')).not.toBeSelected();
      await expect(await findButton('Test Connection')).toBeDisabled();
      await fill('input[aria-label="Endpoint"]', 'https://example.invalid/emmm-qa');
      await fill('input[aria-label="API key"]', '   ');
      await clickButton('Save Config');
      await expectSettings(
        'Blank key changed credential presence',
        (settings) =>
          settings.ai.base_url === 'https://example.invalid/emmm-qa' &&
          settings.ai.has_api_key === false,
      );
      await fill('input[aria-label="API key"]', DUMMY_QA_KEY);
      await click('button[title="Show Key"]');
      await expect(await $('input[aria-label="API key"]')).toHaveAttribute('type', 'text');
      await click('button[title="Hide Key"]');
      await expect(await $('input[aria-label="API key"]')).toHaveAttribute('type', 'password');
      dummyKeyAttempted = true;
      await clickButton('Save Config');
      await expectSettings(
        'Dummy QA credential was not stored',
        (settings) => settings.ai.has_api_key,
      );
      await expect(await $('input[aria-label="API key"]')).toHaveValue('');
      await clickButton('Remove Key');
      await expectSettings(
        'Dummy QA credential was not removed',
        (settings) => !settings.ai.has_api_key,
      );
      await expect(await findButton('Test Connection')).toBeDisabled();
      dummyKeyAttempted = false;
    },
  );

  qaCase('reports local Catalog refresh for the installed or missing fixture pack', async () => {
    await openTab('catalog');
    const before = await invokeInApp<CatalogPackStatus>('get_catalog_pack_status');
    await clickButton('Refresh catalog');
    await expect(await findButton('Refresh catalog')).toBeEnabled();
    if (before.state === 'not_installed') {
      await expect(await $('section[aria-labelledby="catalog-assets-heading"]')).toHaveText(
        /No catalog pack installed\./,
      );
      await expect(await $(SETTINGS)).toHaveText(
        /The installed catalog pack could not be validated\./,
      );
      expect((await invokeInApp<CatalogPackStatus>('get_catalog_pack_status')).state).toBe(
        'not_installed',
      );
      return;
    }
    await expect(await $('section[aria-labelledby="catalog-assets-heading"]')).not.toHaveText(
      /could not be validated/,
    );
    await expect(await $('section[aria-labelledby="catalog-assets-heading"]')).not.toHaveText(
      /status is unavailable/,
    );
  });

  qaCase(
    'refreshes Logs and selects each visible level without opening the OS folder',
    async () => {
      await openTab('logs');
      await clickButton('Refresh');
      await expect(await findButton('Refresh')).toBeEnabled();
      for (const level of ['ERROR', 'WARN', 'INFO', 'ALL'] as const) {
        await select('select[aria-label="Filter logs by level"]', level);
        await expect(await $('select[aria-label="Filter logs by level"]')).toHaveValue(level);
        await expect(await $('#logs-heading')).toBeDisplayed();
      }
    },
  );

  qaCase('runs Maintenance and clears only the isolated old thumbnail cache', async () => {
    await openTab('maintenance');
    await clickButton('Run Maintenance');
    await expectToast('Maintenance complete.');
    await clickButton('Clear Old Cache');
    await expectToast('Successfully cleared');
    expect(await fs.readFile(sentinelPath, 'utf8')).toContain('[TextureOverrideMockMod]');
  });

  qaCase('cancels Reset Application Setup without changing owned game registrations', async () => {
    await openTab('maintenance');
    const before = (await readSettings()).games.map((game) => game.id);
    await click('#btn-reset-database');
    await expect(await $(OPEN_DIALOG)).toHaveText(
      /Your mod files and folders on disk will not be deleted\./,
    );
    await clickButton('Cancel', OPEN_DIALOG);
    await expect(await $(OPEN_DIALOG)).not.toExist();
    expect((await readSettings()).games.map((game) => game.id)).toEqual(before);
  });

  for (const dismissal of ['Escape', 'backdrop'] as const) {
    qaCase(
      `dismisses Reset Application Setup with ${dismissal} without changing registrations`,
      async () => {
        await openTab('maintenance');
        const before = (await readSettings()).games.map((game) => game.id);
        await click('#btn-reset-database');
        await $(OPEN_DIALOG).waitForDisplayed({ timeout: 10_000 });
        if (dismissal === 'Escape') {
          await browser.keys('Escape');
        } else {
          // The modal's full-window Close backdrop surrounds its centered box.
          await browser.action('pointer').move({ x: 8, y: 8 }).down().up().perform();
        }
        await expect(await $(OPEN_DIALOG)).not.toExist();
        expect((await readSettings()).games.map((game) => game.id)).toEqual(before);
      },
    );
  }
});
