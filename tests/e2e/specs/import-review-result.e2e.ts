import { $, browser, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import { createMockGame, scheduleMockGameRemoval, type MockGame } from '../support/fixtures.js';
import { seedGameAndOpenDashboard, gotoWorkspaceView } from '../support/app.js';
import { createObject } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';
import type { AppSettings } from '../../../src/shared/api/tauri/bindings.gen.js';

describe('Native import review defaults and result dialog', function () {
  this.timeout(90_000);
  let game: MockGame;
  let inbox: string;
  const sourceName = 'OwnedRobinSummer';

  before(async () => {
    game = await createMockGame('ImportReviewResult');
    const gameId = await seedGameAndOpenDashboard(game);
    await createObject(gameId, 'OwnedRobin', 'Other');
    inbox = path.join(game.root, 'Inbox');
    await fs.mkdir(path.join(inbox, sourceName), { recursive: true });
    await fs.writeFile(
      path.join(inbox, sourceName, 'mod.ini'),
      '[TextureOverrideOwned]\nhash = faceb00c\n',
    );
    const settings = await invokeInApp<AppSettings>('get_settings');
    await invokeInApp('save_settings', {
      settings: {
        ...settings,
        games: settings.games.map((configured) =>
          configured.id === gameId ? { ...configured, ready_to_move_path: inbox } : configured,
        ),
      },
    });
    await browser.setWindowSize(1280, 900);
  });

  after(() => {
    if (game) scheduleMockGameRemoval(game);
  });

  afterEach(async function () {
    if (this.currentTest?.state === 'failed') {
      await browser.saveScreenshot('.audit-output/import-review-result-20261017/failure.png');
      console.error(await $('body').getText());
    }
  });

  it('defaults the high match to Proceed then replaces the table with a dismissible result', async () => {
    await gotoWorkspaceView('mod-inbox');
    const review = await $('button*=Review selected');
    await review.waitForClickable({ timeout: 20_000 });
    await review.click();
    const row = await $('dialog[open] tbody tr[data-index="0"]');
    await row.waitForDisplayed({ timeout: 30_000 });
    expect(await row.getText()).toContain(sourceName);
    expect(await row.$('input[type="checkbox"]').isSelected()).toBe(true);
    expect(await row.$('button=Proceed').getAttribute('class')).toContain('btn-success');
    await browser.saveScreenshot('.audit-output/import-review-result-20261017/review.png');
    const commit = await $('button=Import 1 item(s)');
    await commit.waitForClickable({ timeout: 20_000 });
    await commit.click();
    const result = await $('dialog[aria-labelledby="import-result-title"]');
    await result.waitForDisplayed({ timeout: 45_000 });
    expect(await result.getText()).toContain('Import complete');
    expect(await result.getText()).toContain('Moved 1,');
    expect(await result.$('table').isExisting()).toBe(false);
    expect(await result.$('input').isExisting()).toBe(false);
    expect(await result.$('button=High').isExisting()).toBe(false);
    const placed = await fs.readdir(path.join(game.modsPath, 'OwnedRobin'));
    expect(placed.some((name) => name.includes(sourceName))).toBe(true);
    await browser.saveScreenshot('.audit-output/import-review-result-20261017/result.png');
    await browser.keys('Escape');
    await result.waitForDisplayed({ reverse: true, timeout: 10_000 });
  });
});
