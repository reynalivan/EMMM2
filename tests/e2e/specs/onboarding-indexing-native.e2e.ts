import { browser, $, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import {
  createMockGame,
  addMockMods,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { invokeInApp } from '../support/ipc.js';
import { getObjects, waitForWorkspaceCoreReady } from '../support/data.js';
import type {
  AppSettings,
  OnboardingIndexingBackgroundStatus,
} from '../../../src/shared/api/tauri/bindings.gen.js';
import type {
  DiskReconcileProgress,
  OnboardingIndexingSnapshotProgress,
} from '../../../src/shared/api/tauri/events.js';

const IDENTIFIER = 'com.reynalivan.emmm.e2e';
const FIRST_MOD_COUNT = 6000;
const SECOND_MOD_COUNT = 80;
const pickerDirectory = process.env.EMMM_E2E_NATIVE_PICKER_REQUEST_DIR;

interface NativeProgressProbe {
  snapshots: OnboardingIndexingSnapshotProgress[];
  disk: DiskReconcileProgress[];
  ui: Array<{ at: number; text: string; now: string | null; total: string | null }>;
  stop: Array<() => void>;
}

interface ProbeWindow extends Window {
  __TAURI__: {
    event: {
      listen: <T>(name: string, callback: (event: { payload: T }) => void) => Promise<() => void>;
    };
  };
  __onboardingProbe: NativeProgressProbe;
}

// The operator bridge selects the real Windows dialog with Computer Use.
// An unavailable native picker is explicitly skipped, never replaced by IPC seeding.
(pickerDirectory ? describe : describe.skip)(
  'Native onboarding with real folder selection and first indexing',
  function () {
    this.timeout(600_000);
    let first: MockGame;
    let second: MockGame;
    let gameId: string;

    before(async () => {
      if (!pickerDirectory || !path.isAbsolute(pickerDirectory)) {
        throw new Error(
          'Native picker operator requires an absolute EMMM_E2E_NATIVE_PICKER_REQUEST_DIR',
        );
      }
      expect(await invokeInApp<string>('plugin:app|identifier')).toBe(IDENTIFIER);
      await fs.mkdir(pickerDirectory, { recursive: true });
      first = await createMockGame('Onboarding_first');
      second = await createMockGame('Onboarding_second');
      await fs.writeFile(
        path.join(pickerDirectory, 'fixtures.json'),
        JSON.stringify([first, second]),
      );
      await addMockMods(
        first,
        'HeavyObject',
        Array.from({ length: FIRST_MOD_COUNT }, (_, i) => `Mod${i}`),
      );
      await addMockMods(
        second,
        'BackgroundObject',
        Array.from({ length: SECOND_MOD_COUNT }, (_, i) => `Mod${i}`),
      );
      const settings = await invokeInApp<AppSettings>('get_settings');
      if (!settings.hotkeys) throw new Error('Native fixture settings omitted hotkeys');
      await invokeInApp('save_settings', {
        settings: { ...settings, language: 'en', hotkeys: { ...settings.hotkeys, enabled: false } },
      });
      await browser.url('http://tauri.localhost/');
      await $('#btn-manual-setup').waitForDisplayed({ timeout: 20_000 });
      await fs.mkdir(pickerDirectory, { recursive: true });
    });

    after(async () => {
      if (first) scheduleMockGameRemoval(first);
      if (second) scheduleMockGameRemoval(second);
    });

    async function selectNativeFolder(game: MockGame, ordinal: number): Promise<void> {
      if (!pickerDirectory) throw new Error('Native picker bridge not configured');
      const buildMode = process.env.EMMM_E2E_BUILD_MODE;
      if (buildMode !== 'debug' && buildMode !== 'release') {
        throw new Error('Native picker requires the validated E2E build mode');
      }
      const token = `picker-${ordinal}`;
      const complete = path.join(pickerDirectory, `${token}.complete.json`);
      await fs.writeFile(
        path.join(pickerDirectory, `${token}.request.json`),
        JSON.stringify({
          token,
          path: game.root,
          fixtureRoot: game.fixtureRoot,
          cleanupToken: game.cleanupToken,
          nativeExecutable: path.resolve('src-tauri', 'target', buildMode, 'emmm-e2e.exe'),
          dialogTitle: 'Select 3DMigoto game folder',
        }),
      );
      await (await $('button=Browse')).click();
      const frame = await browser.execute(() => ({
        url: window.location.href,
        headings: [...document.querySelectorAll('h2')].map((node) => node.textContent),
        path: (document.querySelector('input[readonly]') as HTMLInputElement | null)?.value ?? null,
      }));
      await fs.writeFile(path.join(pickerDirectory, `${token}.ui.json`), JSON.stringify(frame));
      console.info(`NATIVE_PICKER_REQUEST ${token}`);
      await browser.waitUntil(
        async () => {
          try {
            const result: { token: string; path: string } = JSON.parse(
              await fs.readFile(complete, 'utf8'),
            );
            return (
              result.token === token && path.normalize(result.path) === path.normalize(game.root)
            );
          } catch (error) {
            if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return false;
            throw error;
          }
        },
        {
          timeout: 300_000,
          interval: 250,
          timeoutMsg: 'Real Windows folder selection was not completed',
        },
      );
      await expect(await $('input[readonly]')).toHaveValue(game.root);
      await (await $('button[type="submit"]')).click();
      await $('button=Confirm').waitForDisplayed({ timeout: 20_000 });
    }

    it('finishes nonempty onboarding, exposes first-game progress, and indexes the second game in background', async () => {
      await (await $('#btn-manual-setup')).click();
      await (await $('select')).selectByAttribute('value', 'GIMI');
      await selectNativeFolder(first, 1);
      await (await $('button=Add Another')).click();
      await (await $('select')).selectByAttribute('value', 'GIMI');
      await selectNativeFolder(second, 2);

      await browser
        .executeAsync(async (done: (result: string | null) => void) => {
          const scope = window as unknown as ProbeWindow;
          const probe: NativeProgressProbe = { snapshots: [], disk: [], ui: [], stop: [] };
          scope.__onboardingProbe = probe;
          try {
            probe.stop.push(
              await scope.__TAURI__.event.listen<OnboardingIndexingSnapshotProgress>(
                'onboarding_indexing:snapshot_progress',
                ({ payload }) => probe.snapshots.push(payload),
              ),
            );
            probe.stop.push(
              await scope.__TAURI__.event.listen<DiskReconcileProgress>(
                'disk_reconcile:progress',
                ({ payload }) => probe.disk.push(payload),
              ),
            );
            const record = () => {
              const container = document.querySelector('[aria-live][aria-busy="true"]');
              if (!container) return;
              const bar = container.querySelector('[role="progressbar"]');
              probe.ui.push({
                at: performance.now(),
                text: container.textContent ?? '',
                now: bar?.getAttribute('aria-valuenow') ?? null,
                total: bar?.getAttribute('aria-valuemax') ?? null,
              });
            };
            const observer = new MutationObserver(record);
            observer.observe(document.getElementById('root') ?? document.body, {
              childList: true,
              subtree: true,
              characterData: true,
              attributes: true,
              attributeFilter: ['aria-valuenow'],
            });
            probe.stop.push(() => observer.disconnect());
            done(null);
          } catch (error) {
            done(String(error));
          }
        })
        .then((error) => {
          if (error) throw new Error(`Native probe registration failed: ${error}`);
        });

      const started = performance.now();
      await (await $('button=Confirm')).click();
      await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: 90_000 });
      const homeWallMs = performance.now() - started;
      const settings = await invokeInApp<AppSettings>('get_settings');
      const firstConfig = settings.games.find(
        (game) => path.normalize(game.mod_path) === path.normalize(first.modsPath),
      );
      const secondConfig = settings.games.find(
        (game) => path.normalize(game.mod_path) === path.normalize(second.modsPath),
      );
      if (!firstConfig || !secondConfig)
        throw new Error('Onboarding did not persist both actual UI selections');
      gameId = firstConfig.id;
      expect(settings.active_game_id).toBe(gameId);
      await waitForWorkspaceCoreReady(gameId, 90_000);
      expect(
        (await getObjects(gameId)).find((object) => object.name === 'HeavyObject')?.mod_count,
      ).toBe(FIRST_MOD_COUNT);
      await browser.waitUntil(
        async () =>
          (
            await invokeInApp<OnboardingIndexingBackgroundStatus[]>(
              'get_onboarding_indexing_background_status',
            )
          ).some((session) =>
            session.games.some(
              (game) => game.game_id === secondConfig.id && game.phase === 'Ready',
            ),
          ),
        { timeout: 90_000, timeoutMsg: 'Second game did not complete background indexing' },
      );

      expect(
        (await getObjects(secondConfig.id)).some((object) => object.mod_count === SECOND_MOD_COUNT),
      ).toBe(true);

      const probe = await browser.execute(() => {
        const scope = window as unknown as ProbeWindow;
        for (const stop of scope.__onboardingProbe.stop) stop();
        return {
          snapshots: scope.__onboardingProbe.snapshots,
          disk: scope.__onboardingProbe.disk,
          ui: scope.__onboardingProbe.ui,
        };
      });
      const firstProgress = probe.snapshots.filter((event) => event.game_id === gameId);
      expect(
        firstProgress.some(
          (event) =>
            event.phase === 'Classifying' &&
            event.completed_roots < event.total_roots &&
            event.folders_classified > 0,
        ),
      ).toBe(true);
      expect(probe.ui.some((state) => state.text.includes('folders classified'))).toBe(true);
      const ownDisk = probe.disk.filter(
        (event) => event.game_id === gameId && event.reason === 'OnboardingCompleted',
      );
      expect(ownDisk.length).toBeGreaterThan(0);
      expect(
        ownDisk.every((event) =>
          event.run_id.startsWith(`${firstProgress[0]?.session_id}:${gameId}`),
        ),
      ).toBe(true);
      expect(ownDisk.some((event) => event.phase === 'Completed')).toBe(true);
      const projection = ownDisk.filter(
        (event) => event.phase === 'Projecting' && event.total_units !== null,
      );
      expect(
        projection.some(
          (event) =>
            event.completed_units === FIRST_MOD_COUNT && event.total_units === FIRST_MOD_COUNT,
        ),
      ).toBe(true);
      expect(
        projection.some(
          (event) => event.completed_units > 0 && event.completed_units < FIRST_MOD_COUNT,
        ),
      ).toBe(true);
      expect(
        projection.every(
          (event, index) =>
            index === 0 || event.completed_units >= projection[index - 1].completed_units,
        ),
      ).toBe(true);
      const finalizing = ownDisk.findIndex((event) => event.phase === 'Finalizing');
      expect(finalizing).toBeGreaterThan(
        ownDisk.findIndex((event) => event.phase === 'Projecting'),
      );
      expect(ownDisk[finalizing].total_units).toBeNull();
      expect(
        probe.ui.some(
          (state) =>
            state.text.includes('mods processed') &&
            state.total === String(FIRST_MOD_COUNT) &&
            Number(state.now) > 0,
        ),
      ).toBe(true);
      console.info(
        `NATIVE_ONBOARDING first_mods=${FIRST_MOD_COUNT} second_mods=${SECOND_MOD_COUNT} home_wall_ms=${Math.round(homeWallMs)} snapshot_events=${firstProgress.length}`,
      );
      if (pickerDirectory)
        await fs.writeFile(
          path.join(pickerDirectory, 'native-progress.json'),
          JSON.stringify({ homeWallMs, ...probe }, null, 2),
        );
    });

    it('retains the first index and active selection after a native process restart', async () => {
      if (!gameId) throw new Error('First onboarding did not finish; restart cannot be validated');
      await browser.reloadSession();
      await browser.url('http://tauri.localhost/');
      await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: 30_000 });
      await waitForWorkspaceCoreReady(gameId, 60_000);
      expect((await invokeInApp<AppSettings>('get_settings')).active_game_id).toBe(gameId);
      expect(
        (await getObjects(gameId)).find((object) => object.name === 'HeavyObject')?.mod_count,
      ).toBe(FIRST_MOD_COUNT);
    });
  },
);
