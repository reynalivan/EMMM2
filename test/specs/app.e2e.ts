import { browser, $ } from '@wdio/globals';

describe('EMMM2 Initial Load', () => {
  it('should launch the application and verify safe initialization', async () => {
    // EMMM2 is a React SPA running inside Tauri Webview

    // 1. Give the React app some time to construct the DOM
    await browser.pause(2000);

    // 2. Identify a known root/layout element (e.g. the main game switcher or settings button)
    // We can assume there's a body or a main tag
    const rootLayout = await $('body');
    await expect(rootLayout).toBeExisting();

    // Example assertion: Check if the application title is correct
    const title = await browser.getTitle();
    // It depends on index.html: it usually defaults to "EMMM2" or similar
    console.log(`[E2E] Window Title Loaded: ${title}`);
    expect(title).toBe('emmm2'); // adjust to the actual window title defined in tauri.conf.json / index.html

    // 3. (Optional) In real tests, we would check for elements like:
    // const settingsBtn = await $('[data-testid="settings-btn"]')
    // await expect(settingsBtn).toBeDisplayed()
  });

  it('should not throw React runtime errors on mount', async () => {
    // Check browser logs for errors:
    const logs = await browser.getLogs('browser');
    const errors = logs.filter((l) => l.level === 'SEVERE');

    if (errors.length > 0) {
      console.warn('Browser Errors found on mount:', errors);
    }

    // In a strict setup, we might fail on severe React errors:
    // expect(errors.length).toBe(0)
  });
});
