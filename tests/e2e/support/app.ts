import { browser, $ } from '@wdio/globals';
import { invokeInApp } from './ipc.js';
import type { MockGame } from './fixtures.js';
import { waitForWorkspaceCoreReady } from './data.js';
import type { AppSettings } from '../../../src/shared/api/tauri/bindings.gen.js';

const APP_URL = 'http://tauri.localhost/';

/** Boots the app at its root URL and waits for React to mount. */
export async function bootApp(): Promise<void> {
  await browser.url(APP_URL);
  const root = await $('#root');
  await root.waitForExist({ timeout: 20000 });
}

/**
 * Seeds a game via IPC (bypassing the native picker), then reboots so the
 * startup config check routes to the dashboard. Returns the created game id.
 */
export async function seedGameAndOpenDashboard(game: MockGame): Promise<string> {
  return (await seedGamesAndOpenDashboard([{ game }]))[0];
}

/** Seeds multiple isolated games while activating only the first/core game. */
export async function seedGamesAndOpenDashboard(
  fixtures: ReadonlyArray<{ game: MockGame; name?: string }>,
  coreReadyTimeout = 30_000,
): Promise<string[]> {
  if (fixtures.length === 0) {
    throw new Error('At least one mock game is required');
  }
  await bootApp();
  const settings = await invokeInApp<AppSettings>('get_settings');
  if (!settings.hotkeys) throw new Error('Native fixture settings omitted hotkey configuration');
  await invokeInApp('save_settings', {
    settings: {
      ...settings,
      language: 'en',
      hotkeys: { ...settings.hotkeys, enabled: false },
      diagnostics: { ...settings.diagnostics, telemetry_enabled: false },
    },
  });

  // `add_game_manual` only validates the folder and returns a candidate — it
  // does not persist. `save_onboarding_games` is what writes it to settings,
  // which is the same order the onboarding UI uses.
  const created = [];
  for (const fixture of fixtures) {
    const candidate = await invokeInApp<{ id: string; name: string }>('add_game_manual', {
      gameType: 'GIMI',
      path: fixture.game.root,
    });
    created.push(fixture.name ? { ...candidate, name: fixture.name } : candidate);
  }
  await invokeInApp('save_onboarding_games', { games: created });
  await invokeInApp('set_active_game', { gameId: created[0].id });

  // Reboot: AppRouter re-runs checkConfigStatus → HasConfig → /dashboard.
  await browser.url(APP_URL);
  const dashboard = await $('[data-testid="dashboard-layout"]');
  await dashboard.waitForExist({ timeout: 25000 });
  await waitForWorkspaceCoreReady(created[0].id, coreReadyTimeout);

  return created.map((game) => game.id);
}

/**
 * Opens the App Menu popover and clicks a nav item (`dashboard`, `mods`,
 * `collections`, `settings`, `storage-optimizer`) to switch the workspace view.
 */
export async function gotoWorkspaceView(view: string): Promise<void> {
  const appMenu = await $('button[title="App Menu"]');
  await appMenu.waitForDisplayed({ timeout: 5000 });
  await browser.waitUntil(() => appMenu.isEnabled(), {
    timeout: 5000,
    timeoutMsg: 'App Menu stayed disabled',
  });
  await appMenu.click();
  const navItem = await $(`[data-testid="nav-${view}"]`);
  await navItem.waitForDisplayed({ timeout: 3000 });
  await browser.waitUntil(() => navItem.isEnabled(), {
    timeout: 3000,
    timeoutMsg: `Navigation item ${view} stayed disabled`,
  });
  await navItem.click();

  const activeView = await $(`[data-testid="dashboard-layout"][data-workspace-view="${view}"]`);
  await activeView.waitForExist({ timeout: 5000 });
}
