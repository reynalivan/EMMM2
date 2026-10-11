import { $, browser, expect } from '@wdio/globals';
import { Key } from 'webdriverio';
import {
  createMockGame,
  addMockMod,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { seedGamesAndOpenDashboard, gotoWorkspaceView } from '../support/app.js';
import { invokeInApp } from '../support/ipc.js';
import type { AppSettings } from '../../../src/shared/api/tauri/bindings.gen.js';

const UI_TIMEOUT = 15_000;
const COMPACT_WIDTH = 1100;
const DESKTOP_HEIGHT = 820;
const FIRST_NAME = 'Shell Lifecycle A';
const SECOND_NAME = 'Shell Lifecycle B';
const EDITED_NAME = 'Shell Lifecycle A edited';

interface HeldSave {
  entered: boolean;
  release: () => void;
  restore: () => void;
}
interface HoldWindow extends Window {
  __TAURI_INTERNALS__: {
    callbacks: Map<number, (response: unknown) => unknown>;
  };
  __EMMM_SHELL_HELD_SAVE__?: HeldSave;
}

describe('Native Settings and compact shell lifecycle', function () {
  this.timeout(90_000);
  let first: MockGame;
  let second: MockGame;
  let firstId: string;
  let secondId: string;

  before(async () => {
    first = await createMockGame('SettingsShellA');
    second = await createMockGame('SettingsShellB');
    await addMockMod(first, 'Fixture Object', 'Fixture Mod');
    await addMockMod(second, 'Other Object', 'Other Mod');
    [firstId, secondId] = await seedGamesAndOpenDashboard([
      { game: first, name: FIRST_NAME },
      { game: second, name: SECOND_NAME },
    ]);
    await browser.setWindowSize(COMPACT_WIDTH, DESKTOP_HEIGHT);
    const viewport = await browser.execute(() => window.innerWidth);
    if (viewport < 768 || viewport >= 1280)
      throw new Error(`Unexpected compact viewport: ${viewport}`);
  });

  after(async () => {
    await browser.releaseActions();
    await browser.execute(() => {
      const target = window as unknown as HoldWindow;
      target.__EMMM_SHELL_HELD_SAVE__?.release();
      target.__EMMM_SHELL_HELD_SAVE__?.restore();
      delete target.__EMMM_SHELL_HELD_SAVE__;
    });
    if (first) scheduleMockGameRemoval(first);
    if (second) scheduleMockGameRemoval(second);
  });

  afterEach(async () => {
    await browser.releaseActions();
    await browser.keys(Key.Escape);
    await browser.waitUntil(async () => !(await $('dialog[open]').isExisting()), {
      timeout: UI_TIMEOUT,
      timeoutMsg: 'A dialog remained open after the native scenario',
    });
  });

  it('retains visible compact launch actions across Dashboard and Mods navigation', async () => {
    for (let attempt = 0; attempt < 2; attempt++) {
      await gotoWorkspaceView('dashboard');
      await gotoWorkspaceView('mods');
      const more = await $('button[aria-label="More actions"]');
      await more.waitForClickable({ timeout: UI_TIMEOUT });
      await more.click();
      const launchMenu = await $('#topbar-more-launch-portal');
      const play = await launchMenu.$('button=Play');
      await play.waitForDisplayed({ timeout: UI_TIMEOUT });
      await expect(play).toBeEnabled();
      await expect(await launchMenu.$('button=Randomizer (Gacha)')).toBeDisplayed();
      await browser.keys(Key.Escape);
    }
  });

  it('blocks native Escape while a real game save completion is held', async () => {
    await gotoWorkspaceView('settings');
    await (await $('[data-testid="settings-tab-games"]')).click();
    const edit = await $('button[title="Edit Game"]');
    await edit.waitForClickable({ timeout: UI_TIMEOUT });
    await edit.click();
    const submit = await $('[data-testid="game-form-submit"]');
    await submit.waitForDisplayed({ timeout: UI_TIMEOUT });
    await (await $('dialog[open] input[name="name"]')).setValue(EDITED_NAME);
    await (await $('dialog[open] input[name="game_exe"]')).setValue(first.exePath);
    await browser.waitUntil(() => submit.isEnabled(), {
      timeout: UI_TIMEOUT,
      timeoutMsg: 'The owned game draft did not satisfy the native form validation',
    });
    await browser.execute(
      (gameId, editedName) => {
        const target = window as unknown as HoldWindow;
        if (target.__EMMM_SHELL_HELD_SAVE__) throw new Error('Save barrier already installed');
        const callbacks = target.__TAURI_INTERNALS__.callbacks;
        if (!(callbacks instanceof Map) || !Object.isExtensible(callbacks))
          throw new Error('The native callback map cannot hold a save completion');
        const originalSet = callbacks.set;
        const originalDescriptor = Object.getOwnPropertyDescriptor(callbacks, 'set');
        const held: Array<() => void> = [];
        let holding = true;
        const state: HeldSave = {
          entered: false,
          release: () => {
            holding = false;
            for (const complete of held.splice(0)) complete();
          },
          restore: () => {
            holding = false;
            if (originalDescriptor) Object.defineProperty(callbacks, 'set', originalDescriptor);
            else if (!Reflect.deleteProperty(callbacks, 'set'))
              throw new Error('Cannot restore the native callback map');
          },
        };
        target.__EMMM_SHELL_HELD_SAVE__ = state;
        Object.defineProperty(callbacks, 'set', {
          configurable: true,
          writable: true,
          value: (id: number, callback: (response: unknown) => unknown) =>
            originalSet.call(callbacks, id, (response: unknown) => {
              if (
                holding &&
                typeof response === 'object' &&
                response !== null &&
                'settings' in response &&
                typeof response.settings === 'object' &&
                response.settings !== null &&
                'games' in response.settings &&
                Array.isArray(response.settings.games) &&
                response.settings.games.some(
                  (game: unknown) =>
                    typeof game === 'object' &&
                    game !== null &&
                    'id' in game &&
                    'name' in game &&
                    game.id === gameId &&
                    game.name === editedName,
                )
              ) {
                state.entered = true;
                held.push(() => {
                  callback(response);
                });
                return;
              }
              return callback(response);
            }),
        });
      },
      firstId,
      EDITED_NAME,
    );
    try {
      await submit.click();
      await browser.waitUntil(
        () =>
          browser.execute(
            () => (window as unknown as HoldWindow).__EMMM_SHELL_HELD_SAVE__?.entered === true,
          ),
        {
          timeout: UI_TIMEOUT,
          timeoutMsg: 'The real native save did not reach the completion barrier',
        },
      );
      await expect(submit).toBeDisabled();
      await browser.keys(Key.Escape);
      await expect(await $('dialog[open] [data-testid="game-form-submit"]')).toBeDisplayed();
      await browser.execute(() => {
        const target = window as unknown as HoldWindow;
        target.__EMMM_SHELL_HELD_SAVE__?.release();
      });
      await browser.waitUntil(
        async () => !(await $('dialog[open] [data-testid="game-form-submit"]').isExisting()),
        { timeout: UI_TIMEOUT, timeoutMsg: 'Completed game save did not close its own editor' },
      );
      const settings = await invokeInApp<AppSettings>('get_settings');
      expect(settings.games.find((game) => game.id === firstId)?.name).toBe(EDITED_NAME);
      expect(settings.games.find((game) => game.id === secondId)?.name).toBe(SECOND_NAME);
    } finally {
      await browser.execute(() => {
        const target = window as unknown as HoldWindow;
        target.__EMMM_SHELL_HELD_SAVE__?.release();
        target.__EMMM_SHELL_HELD_SAVE__?.restore();
        delete target.__EMMM_SHELL_HELD_SAVE__;
      });
    }
  });

  it('restores global resize styles when keyboard navigation replaces the workspace', async () => {
    await gotoWorkspaceView('mods');
    const handle = await $('[data-testid="resize-handle-left"]');
    await handle.waitForDisplayed({ timeout: UI_TIMEOUT });
    const previous = await browser.execute(() => ({
      cursor: document.body.style.cursor,
      userSelect: document.body.style.userSelect,
    }));
    try {
      await browser
        .action('pointer')
        .move({ origin: handle, duration: 0 })
        .down({ button: 0 })
        .perform(true);
      await browser.waitUntil(
        () => browser.execute(() => document.body.style.cursor === 'col-resize'),
        { timeout: UI_TIMEOUT, timeoutMsg: 'Native pointer did not begin resizing' },
      );
      await browser.execute(() =>
        document.querySelector<HTMLButtonElement>('button[title="App Menu"]')?.focus(),
      );
      await browser.action('key').down(Key.Enter).up(Key.Enter).perform(true);
      await (await $('[data-testid="nav-settings"]')).waitForDisplayed({ timeout: UI_TIMEOUT });
      await browser.execute(() =>
        document.querySelector<HTMLButtonElement>('[data-testid="nav-settings"]')?.focus(),
      );
      await browser.action('key').down(Key.Enter).up(Key.Enter).perform(true);
      await (await $('[data-workspace-view="settings"]')).waitForExist({ timeout: UI_TIMEOUT });
      const current = await browser.execute(() => ({
        cursor: document.body.style.cursor,
        userSelect: document.body.style.userSelect,
      }));
      expect(current).toEqual(previous);
    } finally {
      await browser.releaseActions();
    }
  });
});
