import { browser, $, expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import os from 'os';

describe('Onboarding & Welcome Screen (req-03)', () => {
  let mockGamePath: string;

  before(async () => {
    // Create mock game folder for FTUE manual setup
    const tempDir = os.tmpdir();
    mockGamePath = path.join(tempDir, `EMMM_FTUE_Game_${Date.now()}`);
    await fs.mkdir(mockGamePath, { recursive: true });
    await fs.mkdir(path.join(mockGamePath, 'Mods'), { recursive: true });

    // Core files for validation
    await fs.writeFile(path.join(mockGamePath, 'Fake_Loader.exe'), '');
    await fs.writeFile(path.join(mockGamePath, 'd3dx.ini'), '');
    await fs.writeFile(path.join(mockGamePath, 'd3d11.dll'), '');
  });

  after(async () => {
    // Cleanup
    await fs.rm(mockGamePath, { recursive: true, force: true });
  });

  it('TC-03-08: Finishing Setup transitions app (mounts dashboard)', async () => {
    await browser.url('http://tauri.localhost/');
    await browser.pause(2000);

    // The global `before` in wdio.conf.ts resets the (E2E-only) database, so
    // AppRouter lands on /welcome here rather than skipping to the dashboard.
    const auroraBg = await $('[data-testid="aurora-bg"]');
    await expect(auroraBg).toBeExisting();

    const logo = await $('[data-testid="logo"]');
    await expect(logo).toBeExisting();

    const manualAddBtn = await $('#btn-manual-setup');
    await manualAddBtn.waitForClickable({ timeout: 5000 });
    await manualAddBtn.click();

    console.log('[E2E] On FTUE Manual Setup, bypassing native file picker...');

    const result = (await browser.executeAsync(async (gamePath, done) => {
      interface TauriWindow extends Window {
        __TAURI__: {
          core: {
            invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
          };
        };
      }
      const { invoke } = (window as unknown as TauriWindow).__TAURI__.core;
      try {
        // This is the pair the real FTUE submit runs: `add_game_manual` only
        // validates the folder and hands back a candidate — nothing is stored
        // until `save_onboarding_games` writes it to settings.
        const res = await invoke('add_game_manual', {
          gameType: 'GIMI',
          path: gamePath,
        });
        await invoke('save_onboarding_games', { games: [res] });
        done({ success: true, res });
      } catch (e) {
        done({ success: false, error: String(e) });
      }
    }, mockGamePath)) as { success: boolean; error?: string; res?: unknown };

    if (!result.success) {
      throw new Error(`[E2E] onboarding game setup failed: ${result.error}`);
    }

    console.log('[E2E] Manual game added successfully. Refreshing page...');
    await browser.refresh();
    await browser.pause(5000); // Wait for boot and state init

    // 5. Verify Dashboard Transition
    console.log('[E2E] Waiting for dashboard-layout...');
    const dashboard = await $('[data-testid="dashboard-layout"]');

    try {
      await dashboard.waitForExist({ timeout: 20000 });
      console.log('[E2E] Dashboard layout found!');
    } catch (e) {
      console.error('[E2E] Dashboard layout NOT found after 20s.');
      console.log('[E2E] Current URL:', await browser.getUrl());
      const source = await browser.getPageSource();
      console.log('[E2E] Page Source Snippet:', source.substring(0, 1000));
      throw e;
    }

    expect(await dashboard.isDisplayed()).toBe(true);
  });
});
