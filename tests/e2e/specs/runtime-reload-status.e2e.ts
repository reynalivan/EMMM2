import { $, browser, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import { bootApp, gotoWorkspaceView, seedGameAndOpenDashboard } from '../support/app.js';
import { createMockGame, scheduleMockGameRemoval, type MockGame } from '../support/fixtures.js';
import { invokeInApp } from '../support/ipc.js';
import { waitForWorkspaceCoreReady } from '../support/data.js';
import type {
  AppSettings,
  KeyViewerRuntimeDiagnostics,
} from '../../../src/shared/api/tauri/bindings.gen.js';

const TIMEOUT = 20_000;

describe('Native runtime reload status with an optional executable', () => {
  let game: MockGame;
  let gameId: string;

  async function diagnostics(): Promise<KeyViewerRuntimeDiagnostics> {
    return invokeInApp('get_keyviewer_runtime_diagnostics', { gameId });
  }

  before(async () => {
    await bootApp();
    expect(await invokeInApp<string>('plugin:app|identifier')).toBe('com.reynalivan.emmm.e2e');
    const settings = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', {
      settings: {
        ...settings,
        keyviewer: { ...settings.keyviewer, enabled: false },
        hotkeys: { ...settings.hotkeys, enabled: false, preset_status_overlay_enabled: false },
      },
    });
    game = await createMockGame('RuntimeReloadStatus');
    await fs.writeFile(
      path.join(game.root, 'd3dx.ini'),
      '[Include]\ninclude_recursive = Mods\ninclude = Core/GIMI/main.ini\n[Hunting]\nreload_config = Ctrl+F11\n',
    );
    gameId = await seedGameAndOpenDashboard(game);
    await browser.setWindowSize(1280, 820);
  });

  after(() => {
    if (game) scheduleMockGameRemoval(game);
  });

  it('does not show a false reload warning for an unchanged disabled overlay', async () => {
    const settings = await invokeInApp<AppSettings>('get_settings');
    expect(settings.games.find((entry) => entry.id === gameId)?.game_exe).toBeNull();
    await browser.waitUntil(async () => (await diagnostics()).reload === 'not_required', {
      timeout: TIMEOUT,
      timeoutMsg: 'No-op runtime sync did not settle without reload',
    });
    expect((await diagnostics()).publication).toBe('unchanged');
    await expect(await $('[data-testid="runtime-sync-status"]')).not.toExist();
  });

  it('offers Home setup and routes standalone Play to Games with an optional executable', async () => {
    const setup = await $('button=Set up executable');
    await setup.waitForDisplayed({ timeout: TIMEOUT });
    await setup.click();
    await expect(await $('[data-testid="settings-tab-games"]')).toHaveAttribute(
      'aria-current',
      'page',
    );
    await $('button[title="Edit Game"]').click();
    const executable = await $('input[name="game_exe"]');
    await executable.waitForDisplayed({ timeout: TIMEOUT });
    await executable.setValue('   ');
    await $('[data-testid="game-form-submit"]').click();
    await executable.waitForDisplayed({ reverse: true, timeout: TIMEOUT });
    expect(
      (await invokeInApp<AppSettings>('get_settings')).games.find((entry) => entry.id === gameId)
        ?.game_exe,
    ).toBeNull();

    await gotoWorkspaceView('dashboard');
    const play = await $('button=Quick Play');
    await play.waitForDisplayed({ timeout: TIMEOUT });
    await play.click();
    await expect(await $('[data-testid="settings-tab-games"]')).toHaveAttribute(
      'aria-current',
      'page',
    );
    await gotoWorkspaceView('dashboard');
  });

  it('retains diagnostic reload state without a topbar notice across sync and navigation', async () => {
    const settings = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', {
      settings: { ...settings, keyviewer: { ...settings.keyviewer, enabled: true } },
    });
    await browser.waitUntil(async () => (await diagnostics()).reload === 'manual', {
      timeout: TIMEOUT,
      timeoutMsg: 'Publication for unknown game state did not retain its reload hint',
    });
    const published = await diagnostics();
    expect(published.publication).toBe('published');
    expect(published.reload_binding).toBe('Ctrl+F11');
    const saved = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_hotkey_configuration', {
      expectedRevision: saved.revision,
      hotkeys: saved.hotkeys,
      keyviewer: saved.keyviewer,
    });
    let unchanged = await diagnostics();
    await browser.waitUntil(
      async () => {
        unchanged = await diagnostics();
        return unchanged.last_sync_unix_ms !== published.last_sync_unix_ms;
      },
      {
        timeout: TIMEOUT,
        timeoutMsg: 'Settings sync did not record a new runtime diagnostic snapshot',
      },
    );
    expect(unchanged.publication).toBe('unchanged');
    expect(unchanged.reload).toBe('manual');
    for (const view of ['dashboard', 'mods']) {
      await gotoWorkspaceView(view);
      await expect(await $('[data-testid="runtime-sync-status"]')).not.toExist();
    }
    expect(
      await fs.readFile(path.join(game.modsPath, '.emmm_data', 'KeyViewer.ini'), 'utf8'),
    ).toContain('EMMM-Artifact');
  });

  it('settles reload diagnostics after a current sync confirms the owned game is stopped', async () => {
    const settings = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', {
      settings: {
        ...settings,
        games: settings.games.map((entry) =>
          entry.id === gameId ? { ...entry, game_exe: game.exePath } : entry,
        ),
      },
    });
    await waitForWorkspaceCoreReady(gameId, TIMEOUT);
    const before = await diagnostics();
    await invokeInApp('retry_runtime_sync', { gameId });
    await browser.waitUntil(
      async () => {
        const current = await diagnostics();
        return (
          current.last_sync_unix_ms !== before.last_sync_unix_ms &&
          current.reload === 'not_required'
        );
      },
      {
        timeout: TIMEOUT,
        timeoutMsg: 'Confirmed-stopped publication did not clear the reload hint',
      },
    );
    await $('[data-testid="runtime-sync-status"]').waitForExist({
      reverse: true,
      timeout: TIMEOUT,
    });
  });
});
