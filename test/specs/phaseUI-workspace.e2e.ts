import { browser, $, expect } from '@wdio/globals';
import { createMockGame, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard, gotoWorkspaceView } from '../support/app.js';

/**
 * Fase UI — Workspace layout & navigation (tc-05 / tc-06 / tc-16 presence).
 * Robust UI-level checks only: navigate via the App Menu and assert the 3-pane
 * workspace landmarks mount. Pixel-drag resize, drag-rect select, and mobile
 * viewport gestures stay [manual-smoke] — inherently flaky in E2E.
 */
describe('Fase UI — Workspace Layout & Navigation', () => {
  let game: MockGame;

  before(async () => {
    game = await createMockGame('PhaseUI');
    await seedGameAndOpenDashboard(game);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-05-12: Mods workspace renders the 3-pane layout landmarks', async () => {
    await gotoWorkspaceView('mods');

    const desktop = await $('[data-testid="workspace-desktop"]');
    await desktop.waitForExist({ timeout: 8000 });

    await expect(await $('[data-testid="workspace-left"]')).toExist();
    await expect(await $('[data-testid="workspace-main"]')).toExist();
    await expect(await $('[data-testid="workspace-right"]')).toExist();
    await expect(await $('[data-testid="object-list-panel"]')).toExist();
    await expect(await $('[data-testid="resize-handle-left"]')).toExist();
  });

  it('TC-05-13: Switching dashboard ↔ mods mounts/unmounts the workspace', async () => {
    await gotoWorkspaceView('dashboard');
    await browser.waitUntil(
      async () => !(await $('[data-testid="workspace-desktop"]').isExisting()),
      { timeout: 8000, timeoutMsg: 'workspace still mounted on dashboard view' },
    );

    await gotoWorkspaceView('mods');
    const desktop = await $('[data-testid="workspace-desktop"]');
    await desktop.waitForExist({ timeout: 8000 });
    await expect(desktop).toExist();
  });
});
