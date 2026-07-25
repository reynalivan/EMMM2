import { browser, $ } from '@wdio/globals';
import { invokeInApp } from './ipc.js';
import type { MockGame } from './fixtures.js';

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
export async function seedGameAndOpenDashboard(
  game: MockGame,
  gameType: number | string = 'Genshin',
): Promise<string> {
  await bootApp();

  const created = await invokeInApp<{ id: string }>('add_game_manual', {
    gameType,
    path: game.root,
  });
  await invokeInApp('set_active_game', { gameId: created.id });

  // Reboot: AppRouter re-runs checkConfigStatus → HasConfig → /dashboard.
  await browser.url(APP_URL);
  const dashboard = await $('[data-testid="dashboard-layout"]');
  await dashboard.waitForExist({ timeout: 25000 });

  return created.id;
}

/**
 * Opens the App Menu popover and clicks a nav item (`dashboard`, `mods`,
 * `collections`, `settings`, `storage-optimizer`) to switch the workspace view.
 */
export async function gotoWorkspaceView(view: string): Promise<void> {
  const appMenu = await $('button[title="App Menu"]');
  await appMenu.waitForClickable({ timeout: 5000 });
  await appMenu.click();
  const navItem = await $(`[data-testid="nav-${view}"]`);
  await navItem.waitForClickable({ timeout: 3000 });
  await navItem.click();
}
