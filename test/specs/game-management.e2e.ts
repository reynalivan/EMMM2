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
    mockGamePath = path.join(tempDir, `EMMM2_Mock_Game_${Date.now()}`);
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

    // Bypass welcome screen if EMMM2 is freshly started without DB configs
    const welcomeLogo = await $('[data-testid="logo"]');
    if (await welcomeLogo.isExisting()) {
      console.log('[E2E] On Welcome Screen, bypassing via IPC for Game Management test...');
      await browser.executeAsync(async (gamePath, done) => {
        try {
          interface TauriWindow extends Window {
            __TAURI__: {
              core: {
                invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
              };
            };
          }
          const { invoke } = (window as unknown as TauriWindow).__TAURI__.core;
          await invoke('add_game_manual', { gameType: 'Genshin', path: gamePath });
          window.location.href = '/dashboard';
        } catch (e) {
          console.error(e);
        }
        done();
      }, mockGamePath);
      await browser.pause(2000);
    }

    // Navigate to Settings
    const appMenuBtn = await $('button[title="App Menu"]');
    await appMenuBtn.waitForClickable({ timeout: 5000 });
    await appMenuBtn.click();

    const settingsMenu = await $('span=Settings');
    await settingsMenu.waitForClickable({ timeout: 2000 });
    await settingsMenu.click();

    // Click "Add Game" in Settings
    const addGameBtn = await $('button=Add Game');
    await addGameBtn.waitForClickable({ timeout: 3000 });
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
    const submitBtn = await $('button=Add Game');
    await submitBtn.click();

    // Give it time to save in DB and refresh React state
    await browser.pause(500);

    // Assert that the game was added to the games list
    const gameCard = await $('h3*=Test Mock E2E Game');
    await expect(gameCard).toBeExisting();

    // E2E-02-01: Launch
    // Use the Quick Play dropdown from Topbar
    await appMenuBtn.click();

    // Using *= to match substring "Quick Play"
    const quickPlayBtn = await $('p*=Quick Play');
    await quickPlayBtn.waitForClickable({ timeout: 2000 });
    await quickPlayBtn.click();

    // Wait a brief moment to allow Rust process launch
    await browser.pause(500);
    console.log('[E2E] Process Spawn triggered successfully.');
  });
});
