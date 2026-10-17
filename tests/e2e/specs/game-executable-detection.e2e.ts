import { $, $$, browser, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import { bootApp, gotoWorkspaceView, seedGameAndOpenDashboard } from '../support/app.js';
import {
  createMockGame,
  addMockMod,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { invokeInApp } from '../support/ipc.js';
import type {
  AppSettings,
  GameExecutableCandidate,
  GameExecutableDiscoveryResult,
} from '../../../src/shared/api/tauri/bindings.gen.js';

const TIMEOUT = 20_000;
const MAX_KEYBOARD_STEPS = 80;

function executableKey(value: string): string {
  const normalized = value.replaceAll('\\', '/').toLowerCase();
  return normalized.startsWith('//?/') ? normalized.slice(4) : normalized;
}

describe('Native installed game executable discovery', function () {
  this.timeout(360_000);
  let fixture: MockGame;
  let onboardingFixture: MockGame;
  let launcherExe: string;
  let gameId: string;
  const paths = new Map<number, string>();

  before(async () => {
    await bootApp();
    expect(await invokeInApp('plugin:app|identifier')).toBe('com.reynalivan.emmm.e2e');
    fixture = await createMockGame('ExecutableDiscovery');
    onboardingFixture = await createMockGame(
      'ExecutableOnboarding',
      path.join(fixture.root, 'GIMI'),
    );
    const launcherDirectory = path.join(fixture.root, 'Resources', 'Bin');
    await fs.mkdir(launcherDirectory, { recursive: true });
    launcherExe = path.join(launcherDirectory, 'XXMI Launcher.exe');
    await fs.writeFile(launcherExe, 'owned synthetic launcher; never launch');
    const profiles = [
      { type: 0, key: 'GIMI', exe: 'GenshinImpact.exe', marker: 'GenshinImpact_Data' },
      { type: 1, key: 'SRMI', exe: 'StarRail.exe', marker: 'StarRail_Data' },
      { type: 2, key: 'WWMI', exe: 'Wuthering Waves.exe', marker: 'Engine' },
      { type: 3, key: 'ZZMI', exe: 'ZenlessZoneZero.exe', marker: 'ZenlessZoneZero_Data' },
      { type: 4, key: 'EFMI', exe: 'Endfield.exe', marker: 'Endfield_Data' },
    ];
    const importers: Record<string, { Importer: { game_folder: string } }> = {};
    for (const profile of profiles) {
      const root = path.join(fixture.root, 'Installed Games', profile.key);
      await fs.mkdir(path.join(root, profile.marker), { recursive: true });
      const executable = path.join(root, profile.exe);
      await fs.writeFile(executable, 'owned synthetic executable; never launch');
      if (profile.type === 2) {
        const shipping = path.join(root, 'Client', 'Binaries', 'Win64');
        await fs.mkdir(shipping, { recursive: true });
        await fs.writeFile(
          path.join(shipping, 'Client-Win64-Shipping.exe'),
          'owned shipping fixture',
        );
      }
      importers[profile.key] = { Importer: { game_folder: root } };
      paths.set(profile.type, executable);
    }
    await fs.writeFile(
      path.join(fixture.root, 'XXMI Launcher Config.json'),
      JSON.stringify({ Importers: importers }),
    );
    gameId = await seedGameAndOpenDashboard(fixture);
    await browser.setWindowSize(1280, 1000);
  });

  after(() => {
    if (fixture) scheduleMockGameRemoval(fixture);
    if (onboardingFixture) scheduleMockGameRemoval(onboardingFixture);
  });

  async function configuredExe() {
    return (await invokeInApp<AppSettings>('get_settings')).games.find((game) => game.id === gameId)
      ?.game_exe;
  }

  async function openEditor() {
    await gotoWorkspaceView('settings');
    await $('[data-testid="settings-tab-games"]').click();
    await $('button[title="Edit Game"]').click();
    await $('input[name="game_exe"]').waitForDisplayed({ timeout: TIMEOUT });
  }

  async function selectFixtureCandidate(scan = true, keyboard = false) {
    if (scan) await $('button=Detect').click();
    const executable = paths.get(0)!;
    let use: WebdriverIO.Element | undefined;
    await browser.waitUntil(
      async () => {
        for (const button of await $$('button=Use this location')) {
          const row = await button.parentElement();
          if ((await row.getText()).includes(executable)) {
            use = button;
            return true;
          }
        }
        return false;
      },
      { timeout: TIMEOUT, timeoutMsg: 'Owned Genshin fixture candidate not offered' },
    );
    await use!.scrollIntoView();
    if (keyboard) {
      let focused = false;
      for (let step = 0; step < MAX_KEYBOARD_STEPS; step += 1) {
        focused = await browser.execute(
          (element: HTMLElement) => document.activeElement === element,
          use!,
        );
        if (focused) break;
        await browser.keys('Tab');
      }
      expect(focused).toBe(true);
      await browser.keys('Enter');
    } else {
      await use!.click();
    }
    await use!.waitForDisplayed({ reverse: true, timeout: TIMEOUT });
    await browser.waitUntil(
      async () => (await $('input[name="game_exe"]').getValue()) === executable,
      { timeout: TIMEOUT },
    );
  }

  it('discovers and validates all five game profiles through allowlisted IPC', async () => {
    const result = await invokeInApp<GameExecutableDiscoveryResult>('discover_game_executables', {
      request: { game_types: [0, 1, 2, 3, 4], xxmi_roots: [fixture.root] },
    });
    for (const [gameType, executable] of paths) {
      const selected = result.candidates.find(
        (candidate) =>
          candidate.game_type === gameType &&
          executableKey(candidate.executable_path) === executableKey(executable),
      );
      expect(selected).toBeDefined();
      expect(selected!.sources).toContain('xxmi_config');
      const validated = await invokeInApp<GameExecutableCandidate>('validate_game_executable', {
        request: { game_type: gameType, executable_path: executable },
      });
      expect(validated.game_type).toBe(gameType);
      paths.set(gameType, validated.executable_path);
      if (gameType === 2)
        expect(validated.process_executable_path).toContain('Client-Win64-Shipping.exe');
    }
    const fromLauncher = await invokeInApp<GameExecutableDiscoveryResult>(
      'discover_game_executables',
      {
        request: { game_types: [4], xxmi_roots: [launcherExe] },
      },
    );
    expect(
      fromLauncher.candidates.some(
        (candidate) => executableKey(candidate.executable_path) === executableKey(paths.get(4)!),
      ),
    ).toBe(true);
  });

  it('applies a candidate only to the draft and Cancel leaves settings unchanged', async () => {
    const before = await configuredExe();
    await openEditor();
    await selectFixtureCandidate();
    expect(await configuredExe()).toBe(before);
    await (await $('dialog[open]')).$('button=Back').click();
    await $('input[name="game_exe"]').waitForDisplayed({ reverse: true, timeout: TIMEOUT });
    expect(await configuredExe()).toBe(before);
  });

  it('persists an explicitly selected executable and still allows optional empty setup', async () => {
    await openEditor();
    await selectFixtureCandidate(true, true);
    const save = await $('[data-testid="game-form-submit"]');
    await save.scrollIntoView();
    await save.click();
    await $('input[name="game_exe"]').waitForDisplayed({ reverse: true, timeout: TIMEOUT });
    expect(await configuredExe()).toBe(paths.get(0));
    await $('button[title="Edit Game"]').click();
    const executableInput = await $('input[name="game_exe"]');
    await executableInput.waitForDisplayed({ timeout: TIMEOUT });
    await executableInput.click();
    await browser.keys(['Control', 'a']);
    await browser.keys('Backspace');
    await expect(executableInput).toHaveValue('');
    await $('[data-testid="game-form-submit"]').click();
    await $('input[name="game_exe"]').waitForDisplayed({ reverse: true, timeout: TIMEOUT });
    expect(await configuredExe()).toBeNull();
  });

  const pickerDirectory = process.env.EMMM_E2E_NATIVE_PICKER_REQUEST_DIR;
  (pickerDirectory ? it : it.skip)(
    'confirms a detected executable through real onboarding',
    async () => {
      if (!pickerDirectory || !path.isAbsolute(pickerDirectory))
        throw new Error('An absolute native picker artifact directory is required');
      await fs.mkdir(pickerDirectory, { recursive: true });
      await addMockMod(onboardingFixture, 'OwnedObject', 'OwnedMod');
      await invokeInApp('reset_database');
      await browser.url('http://tauri.localhost/');
      await $('#btn-manual-setup').waitForDisplayed({ timeout: TIMEOUT });
      await $('#btn-manual-setup').click();
      await $('select').selectByAttribute('value', 'GIMI');
      const token = 'executable-onboarding-picker';
      const buildMode = process.env.EMMM_E2E_BUILD_MODE;
      if (buildMode !== 'debug' && buildMode !== 'release')
        throw new Error('Validated native build mode is required');
      await fs.writeFile(
        path.join(pickerDirectory, `${token}.request.json`),
        JSON.stringify({
          token,
          path: onboardingFixture.root,
          fixtureRoot: fixture.fixtureRoot,
          cleanupToken: fixture.cleanupToken,
          nativeExecutable: path.resolve('src-tauri', 'target', buildMode, 'emmm-e2e.exe'),
          dialogTitle: 'Select 3DMigoto game folder',
        }),
      );
      const folderField = await $('input[readonly]');
      await (await (await folderField.parentElement()).$('button')).click();
      await browser.waitUntil(
        async () => {
          try {
            const result: { token: string; path: string } = JSON.parse(
              await fs.readFile(path.join(pickerDirectory, `${token}.complete.json`), 'utf8'),
            );
            return (
              result.token === token &&
              executableKey(result.path) === executableKey(onboardingFixture.root)
            );
          } catch (error) {
            if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return false;
            throw error;
          }
        },
        {
          timeout: 300_000,
          interval: 250,
          timeoutMsg: 'Real native folder selection was not completed',
        },
      );
      await expect(folderField).toHaveValue(onboardingFixture.root);
      await selectFixtureCandidate();
      await $('form button[type="submit"]').click();
      await $('button=Confirm').waitForDisplayed({ timeout: TIMEOUT });
      await expect(await $('input[name="game_exe"]')).toHaveValue(paths.get(0)!);
      await $('button=Detect executable locations').click();
      await selectFixtureCandidate(false);
      await $('button=Confirm').click();
      await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: 45_000 });
      const saved = await invokeInApp<AppSettings>('get_settings');
      expect(saved.games).toHaveLength(1);
      expect(saved.games[0].game_exe).toBe(paths.get(0));
      expect(executableKey(saved.games[0].mod_path)).toBe(
        executableKey(onboardingFixture.modsPath),
      );
    },
  );

  it('rejects an installation removed after discovery', async () => {
    const executable = paths.get(4)!;
    const moved = `${executable}.moved`;
    await fs.rename(executable, moved);
    try {
      await expect(
        invokeInApp('validate_game_executable', {
          request: { game_type: 4, executable_path: executable },
        }),
      ).rejects.toBeDefined();
    } finally {
      await fs.rename(moved, executable);
    }
  });
});
