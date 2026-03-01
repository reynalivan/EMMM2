import { browser, $, expect } from '@wdio/globals';

describe('App Bootstrap & Initialization (req-01)', () => {
  it('TC-01-02: Happy Path Start - App starts and interactive UI loads within timeout', async () => {
    // Ensure we are on the app URL
    console.log('[E2E] Navigating to http://tauri.localhost/');
    await browser.url('http://tauri.localhost/');

    let title = await browser.getTitle();
    if (!title) {
      console.log('[E2E] Title empty, trying tauri://localhost/');
      await browser.url('tauri://localhost/');
      title = await browser.getTitle();
    }
    console.log(`[E2E] Current Title: ${title}`);

    // Wait for the app to load and the window title to be set
    const root = await $('#root');
    try {
      await root.waitForExist({ timeout: 10000 });
    } catch {
      console.log('[E2E] #root not found, dumping page source...');
      console.log(await browser.getPageSource());
      throw new Error('#root not found');
    }

    await browser.waitUntil(
      async () => {
        const title = await browser.getTitle();
        return title === 'EMMM2';
      },
      { timeout: 15000, timeoutMsg: 'Window title did not load in 15s' },
    );

    const handles = await browser.getWindowHandles();
    console.log('WINDOW HANDLES: ', handles);

    // Sometimes there are background handles, find the one with the title
    for (const handle of handles) {
      await browser.switchToWindow(handle);
      const title = await browser.getTitle();
      if (title === 'EMMM2' || title === 'emmm2') {
        break;
      }
    }

    // Identify the main React root tag - use a longer timeout for the first mount
    const reactRoot = await $('#root');
    await reactRoot.waitForExist({ timeout: 15000 });
    await expect(reactRoot).toExist();

    // Verify window title to ensure Tauri window config loaded correctly
    await browser.waitUntil(
      async () => {
        const title = await browser.getTitle();
        return title === 'EMMM2';
      },
      { timeout: 5000 },
    );

    // Verify some text or component from the dashboard/onboarding
    const body = await $('body');
    const text = await body.getText();
    expect(text).toMatch(/EMMM2|Welcome|Setup/i);
  });
});
