import { browser, $ } from '@wdio/globals';

describe('EMMM Initial Load', () => {
  it('should launch the application and verify safe initialization', async () => {
    // EMMM is a React SPA running inside Tauri Webview

    // 1. Give the React app some time to construct the DOM
    await browser.pause(2000);

    // 2. Identify a known root/layout element (e.g. the main game switcher or settings button)
    // We can assume there's a body or a main tag
    const rootLayout = await $('body');
    await expect(rootLayout).toBeExisting();

    // Example assertion: Check if the application title is correct
    const title = await browser.getTitle();
    // It depends on index.html: it usually defaults to "EMMM" or similar
    console.log(`[E2E] Window Title Loaded: ${title}`);
    expect(title).toBe('emmm'); // adjust to the actual window title defined in tauri.conf.json / index.html

    // 3. (Optional) In real tests, we would check for elements like:
    // const settingsBtn = await $('[data-testid="settings-btn"]')
    // await expect(settingsBtn).toBeDisplayed()
  });

  it('should not throw React runtime errors on mount', async () => {
    const logs = await browser.getLogs('browser');
    const severe = logs.filter(
      (l): l is { level: string; message: string } =>
        typeof l === 'object' && l !== null && (l as { level?: string }).level === 'SEVERE',
    );

    // Gate on genuine app faults only — ignore resource/network noise the
    // webview emits in a debug build.
    const appErrors = severe.filter((l) =>
      /Uncaught|Minified React error|React will try to recreate|Cannot read propert/i.test(
        l.message ?? '',
      ),
    );

    if (severe.length > 0) {
      console.warn('SEVERE browser logs on mount:', severe);
    }
    if (appErrors.length > 0) {
      console.error('Uncaught/React errors on mount:', appErrors);
    }
    expect(appErrors.length).toBe(0);
  });
});
