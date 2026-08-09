import { browser, $ } from '@wdio/globals';

describe('EMMM Initial Load', () => {
  it('should launch the application and verify safe initialization', async () => {
    // The session attaches to a blank webview — nothing is loaded until we
    // navigate, so every spec must open the app URL before asserting on it.
    await browser.url('http://tauri.localhost/');

    const root = await $('#root');
    await root.waitForExist({ timeout: 20000 });

    // Matches the <title> in index.html.
    expect(await browser.getTitle()).toBe('EMMM');

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
