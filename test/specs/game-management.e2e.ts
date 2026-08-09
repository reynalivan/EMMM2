import { browser, $, expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import os from 'os';

describe('Game Management (req-02)', () => {
  let mockGamePath: string;
  let mockModPath: string;
  let mockExePath: string;

  before(async () => {
    // Create mock game folder to safely test file operations
    const tempDir = os.tmpdir();
    mockGamePath = path.join(tempDir, `EMMM_Mock_Game_${Date.now()}`);
    mockModPath = path.join(mockGamePath, 'Mods');
    mockExePath = path.join(mockGamePath, 'Fake_Game_Loader.exe'); // use 'loader' in name as hint for validator

    await fs.mkdir(mockGamePath, { recursive: true });
    await fs.mkdir(mockModPath, { recursive: true });

    // Create required 3DMigoto core files to pass backend validation
    await fs.writeFile(mockExePath, 'mock exe binary content');
    await fs.writeFile(path.join(mockGamePath, 'd3dx.ini'), '[Main]\n');
    await fs.writeFile(path.join(mockGamePath, 'd3d11.dll'), 'mock dll content');
  });

  after(async () => {
    // Cleanup mock folder after test
    await fs.rm(mockGamePath, { recursive: true, force: true });
  });

  it('TC-02-05: Manual Add Game (Settings) & Launch', async () => {
    await browser.url('http://tauri.localhost/');
    await browser.pause(2000);

    // Always seed a persisted game rather than branching on what the previous
    // spec left behind: without one, AppRouter parks on /welcome and the App
    // Menu never mounts. `add_game_manual` alone does not persist — only
    // `save_onboarding_games` writes to settings.
    await browser.executeAsync(async (gamePath, done) => {
      interface TauriWindow extends Window {
        __TAURI__: {
          core: {
            invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
          };
        };
      }
      const { invoke } = (window as unknown as TauriWindow).__TAURI__.core;
      try {
        const game = await invoke('add_game_manual', { gameType: 'GIMI', path: gamePath });
        await invoke('save_onboarding_games', { games: [game] });
      } catch {
        // Already registered from an earlier run — the dashboard is reachable either way.
      }
      done();
    }, mockGamePath);

    await browser.url('http://tauri.localhost/');

    // Navigate to Settings
    const appMenuBtn = await $('button[title="App Menu"]');
    await appMenuBtn.waitForClickable({ timeout: 20000 });
    await appMenuBtn.click();

    const settingsMenu = await $('span=Settings');
    await settingsMenu.waitForClickable({ timeout: 2000 });
    await settingsMenu.click();

    // Both the tab action and the modal submit read "Add Game", and each carries
    // an icon so their text nodes are not exact matches — target the testids.
    const addGameBtn = await $('[data-testid="games-add"]');
    await addGameBtn.waitForClickable({ timeout: 10000 });
    await addGameBtn.click();

    // Fill out the modal
    const nameInput = await $('input[placeholder="e.g. Genshin Impact"]');
    await nameInput.waitForDisplayed({ timeout: 2000 });
    await nameInput.setValue('Test Mock E2E Game');

    const modPathInput = await $('input[placeholder="C:/Games/Genshin Impact/Mods"]');
    await modPathInput.setValue(mockModPath);

    const exeInput = await $('input[placeholder="C:/Games/Genshin Impact/GenshinImpact.exe"]');
    await exeInput.setValue(mockExePath);

    // Submit Game
    const submitBtn = await $('[data-testid="game-form-submit"]');
    await submitBtn.waitForClickable({ timeout: 5000 });
    await submitBtn.click();

    // Assert that the game was added to the games list
    const gameCard = await $('h3*=Test Mock E2E Game');
    await gameCard.waitForExist({ timeout: 10000 });
    await expect(gameCard).toBeExisting();

    // Launching is deliberately NOT driven here: Quick Play spawns the real
    // 3DMigoto loader process, which is on the [manual-smoke] list.
  });
});
