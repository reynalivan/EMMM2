import { browser, $, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import {
  addMockMod,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { seedGamesAndOpenDashboard } from '../support/app.js';
import { getObjects, waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';

describe('Native watcher consolidation', () => {
  const games: MockGame[] = [];
  let firstId: string;
  let secondId: string;

  before(async () => {
    expect(await invokeInApp<string>('plugin:app|identifier')).toBe('com.reynalivan.emmm.e2e');
    games.push(await createMockGame('WatcherActive'));
    games.push(await createMockGame('WatcherInactive'));
    await addMockMod(games[0], 'ActiveObject', 'Initial');
    await addMockMod(games[1], 'InactiveObject', 'Initial');
    [firstId, secondId] = await seedGamesAndOpenDashboard(games.map((game) => ({ game })));
  });

  after(() => {
    for (const game of games) scheduleMockGameRemoval(game);
  });

  function objectFolder(folderPath: string): string | undefined {
    return folderPath.replaceAll('\\', '/').split('/').at(-1);
  }

  async function waitForCount(gameId: string, name: string, count: number): Promise<void> {
    await browser.waitUntil(
      async () => {
        const object = (await getObjects(gameId)).find(
          (candidate) => objectFolder(candidate.folder_path) === name,
        );
        return object?.mod_count === count;
      },
      {
        timeout: 20_000,
        interval: 100,
        timeoutMsg: `Watcher did not project ${name} count=${count}`,
      },
    );
    await waitForWorkspaceCoreReady(gameId);
    await $('[data-testid="dashboard-layout"]').waitForDisplayed();
  }

  it('projects an external create, rename and removal while Ready without manual reconcile', async () => {
    const added = await addMockMod(games[0], 'ActiveObject', 'External');
    await waitForCount(firstId, 'ActiveObject', 2);
    const renamedObject = path.join(games[0].modsPath, 'ActiveObject.v2');
    await fs.rename(path.dirname(added), renamedObject);
    await waitForCount(firstId, 'ActiveObject.v2', 2);
    expect(
      (await getObjects(firstId)).some(
        (object) => objectFolder(object.folder_path) === 'ActiveObject',
      ),
    ).toBe(false);
    await fs.rename(
      path.join(renamedObject, 'External'),
      path.join(games[0].root, 'RemovedExternal'),
    );
    await waitForCount(firstId, 'ActiveObject.v2', 1);
  });

  it('catches up an inactive game and keeps coverage after same-game reuse and switch-back', async () => {
    await addMockMod(games[1], 'InactiveObject', 'BeforeActivation');
    await invokeInApp('set_active_game', { gameId: secondId });
    await waitForCount(secondId, 'InactiveObject', 2);
    await invokeInApp('set_active_game', { gameId: secondId });
    await addMockMod(games[1], 'InactiveObject', 'AfterReuse');
    await waitForCount(secondId, 'InactiveObject', 3);
    await invokeInApp('set_active_game', { gameId: firstId });
    await waitForWorkspaceCoreReady(firstId);
    await addMockMod(games[0], 'ActiveObject.v2', 'AfterSwitchBack');
    await waitForCount(firstId, 'ActiveObject.v2', 2);
  });
});
