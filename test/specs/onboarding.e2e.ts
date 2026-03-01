import { browser, $, expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import os from 'os';

describe('Onboarding & Welcome Screen (req-03)', () => {
  let mockGamePath: string;

  before(async () => {
    // Create mock game folder for FTUE manual setup
    const tempDir = os.tmpdir();
    mockGamePath = path.join(tempDir, `EMMM2_FTUE_Game_${Date.now()}`);
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

    // Force navigation to /welcome to naturally test the FTUE UI components without wiping user databases
    await browser.executeAsync(async (done) => {
      window.location.hash = '';
      window.location.pathname = '/welcome';
      done();
    });

    await browser.pause(1000);

    // Verify Welcome Screen is visible by checking for its unique elements
    const auroraBg = await $('[data-testid="aurora-bg"]');
    await expect(auroraBg).toBeExisting();

    const logo = await $('[data-testid="logo"]');
    await expect(logo).toBeExisting();

    // Click "Add Game Manually"
    const manualAddBtn = await $('#btn-manual-setup');
    await manualAddBtn.waitForClickable({ timeout: 2000 });
    await manualAddBtn.click();

    await browser.pause(500);

    // Now we are on the ManualSetupForm screen
    console.log('[E2E] On FTUE Manual Setup, bypassing native file picker...');

    // We do what `onSubmit` normally does
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
        const res = await invoke('add_game_manual', {
          gameType: 'GIMI',
          path: gamePath,
        });
        done({ success: true, res });
      } catch (e) {
        done({ success: false, error: String(e) });
      }
    }, mockGamePath)) as { success: boolean; error?: string; res?: unknown };

    if (!result.success) {
      throw new Error(`[E2E] add_game_manual failed: ${result.error}`);
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
