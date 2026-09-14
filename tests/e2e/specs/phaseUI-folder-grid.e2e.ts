import { $, expect } from '@wdio/globals';
import { createMockGame, addMockMod, removeMockGame, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard, gotoWorkspaceView } from '../support/app.js';
import { createObject, reconcile } from '../support/data.js';

/**
 * Fase UI — Folder grid interaction (tc-06 / tc-12 click-driven).
 * Object selection opens the FolderGrid (single-click → focusObject →
 * selectedObjectFolderPath), and the grid/list view toggle switches without
 * error. Robust click interactions only; pixel-drag stays [manual-smoke].
 */
describe('Fase UI — Folder Grid Interaction', () => {
  let game: MockGame;
  let gameId: string;

  before(async () => {
    game = await createMockGame('PhaseUIGrid');
    gameId = await seedGameAndOpenDashboard(game);
    // Seed BEFORE entering mods view so ObjectList's first fetch shows the row.
    await createObject(gameId, 'UiGridObj');
    await addMockMod(game, 'UiGridObj', 'UiGridMod');
    await reconcile(gameId);
  });

  after(async () => {
    await removeMockGame(game);
  });

  it('TC-06-01: Selecting an object opens its folder grid', async () => {
    await gotoWorkspaceView('mods');

    const row = await $('[data-object-id]');
    await row.waitForExist({ timeout: 8000 });
    await row.waitForClickable({ timeout: 3000 });
    await row.click();

    const grid = await $('[data-testid="folder-grid"]');
    await grid.waitForExist({ timeout: 8000 });
    await expect(grid).toExist();
  });

  it('TC-12-01: Grid/list view toggle switches without error', async () => {
    // Grid is already open from the previous test's selection.
    const listBtn = await $('[data-testid="view-list"]');
    await listBtn.waitForClickable({ timeout: 5000 });
    await listBtn.click();

    const gridBtn = await $('[data-testid="view-grid"]');
    await gridBtn.waitForClickable({ timeout: 3000 });
    await gridBtn.click();

    // Grid container stays mounted through the toggles.
    await expect(await $('[data-testid="folder-grid"]')).toExist();
  });
});
