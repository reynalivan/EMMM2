import { browser, $, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import { performance } from 'node:perf_hooks';
import { invokeInApp } from '../support/ipc.js';
import { startNativeCpuProfile } from '../support/nativeCpuProfile.js';
import { getObjects, waitForWorkspaceCoreReady } from '../support/data.js';
import {
  READONLY_AUDIT_GAME,
  READONLY_AUDIT_IDENTIFIER,
  readonlyAuditPaths,
  ensureInstalledAppStopped,
} from '../support/readonlyAudit.js';
import type {
  AppSettings,
  WorkspaceStructureViewModel,
} from '../../../src/shared/api/tauri/bindings.gen.js';

interface UiTimingEvent {
  startTime: number;
  duration: number;
}

interface StartupProbe {
  frames: UiTimingEvent[];
  longTasks: UiTimingEvent[];
  animation: number;
  observer: PerformanceObserver | null;
  visible: boolean;
  startedPageMs: number;
  visibilityChanged: () => void;
  recordLongTasks: (entries: PerformanceEntry[]) => void;
}

async function observeCoreReady() {
  let failed = false;
  await browser.waitUntil(
    async () => {
      const snapshot = await invokeInApp<WorkspaceStructureViewModel>('get_workspace_structure', {
        input: {
          filter: {
            game_id: READONLY_AUDIT_GAME,
            search_query: null,
            object_type: null,
            meta_filters: null,
            sort_by: null,
            status_filter: null,
          },
          selected_object_folder_path: null,
          explorer_sub_path: null,
        },
      });
      failed = snapshot.runtime.recovery_status === 'failed';
      return (
        failed ||
        (snapshot.runtime.recovery_status === 'ready' &&
          snapshot.runtime.source_state.status === 'available')
      );
    },
    { timeout: 180_000, interval: 100, timeoutMsg: 'Real library core recovery did not settle' },
  );
  if (failed)
    throw new Error(
      'Real library core recovery failed; preserve logs without repairing real files',
    );
}

describe('Read-only native library startup audit', () => {
  it('measures the first isolated index and subsequent native process restarts', async () => {
    const audit = readonlyAuditPaths();
    if (!process.env.LOCALAPPDATA) throw new Error('LOCALAPPDATA is required');
    const logPath = path.join(
      process.env.LOCALAPPDATA,
      READONLY_AUDIT_IDENTIFIER,
      'logs',
      'emmm.log',
    );
    const measurements: Array<{
      run: number;
      startedUnixMs: number;
      shellWallMs: number;
      shellPageMs: number;
      coreWallMs: number;
      optionalWallMs: number;
      frameGaps: number[];
      longTasks: number[];
      frameEvents: UiTimingEvent[];
      longTaskEvents: UiTimingEvent[];
      frameObservationStartMs: number;
      timeOrigin: number;
      pageVisible: boolean;
      viewport: { width: number; height: number; devicePixelRatio: number };
      homeHasError: boolean;
      timing: string;
      registrations: number;
      sampleKind: 'empty-index-process-start' | 'indexed-process-restart';
      buildMode: string;
      cpuProfile: boolean;
      objectCount: number;
      modCount: number;
      modsPerCoreSecond: number;
      osCache: 'uncontrolled';
    }> = [];
    const release = process.env.EMMM_E2E_BUILD_MODE === 'release';
    if (!release) {
      await browser.url('http://tauri.localhost/');
      expect(await invokeInApp<string>('plugin:app|identifier')).toBe(READONLY_AUDIT_IDENTIFIER);
      await waitForWorkspaceCoreReady(READONLY_AUDIT_GAME, 180_000);
    }
    const sessionStart = Number(process.env.EMMM_AUDIT_SESSION_START_UNIX_MS);
    if (release && (!Number.isFinite(sessionStart) || sessionStart <= 0)) {
      throw new Error('Release startup requires a recorded session launch boundary');
    }
    let expectedModCount: number | null = null;
    let expectedObjectCount: number | null = null;
    for (let run = release ? 0 : 1; run <= 3; run += 1) {
      ensureInstalledAppStopped();
      const oldLog =
        run === 0
          ? await fs.readFile(path.join(audit.output, 'native-before-launch.log'), 'utf8')
          : await fs.readFile(logPath, 'utf8');
      const started =
        run === 0 ? performance.now() - (Date.now() - sessionStart) : performance.now();
      const startedUnixMs = run === 0 ? sessionStart : Date.now();
      if (run !== 0) await browser.reloadSession();
      if (!release) await browser.url('http://tauri.localhost/');
      const stopCpuProfile = await startNativeCpuProfile(audit.output, run);
      let measurementFailed = false;
      try {
        await browser.waitUntil(
          () =>
            browser.execute(() => {
              const scope = window as unknown as { __TAURI__?: { core?: { invoke?: unknown } } };
              return (
                window.location.origin === 'http://tauri.localhost' &&
                typeof scope.__TAURI__?.core?.invoke === 'function'
              );
            }),
          { timeout: 30_000, timeoutMsg: 'Native audit document and IPC bridge not ready' },
        );
        expect(await invokeInApp<string>('plugin:app|identifier')).toBe(READONLY_AUDIT_IDENTIFIER);
        await browser.execute(() => {
          const scope = window as unknown as { __readonlyStartupProbe: StartupProbe };
          let last: number | null = null;
          const probe: StartupProbe = {
            frames: [],
            longTasks: [],
            animation: 0,
            observer: null,
            visible: !document.hidden,
            startedPageMs: performance.now(),
            visibilityChanged: () => {
              last = null;
              probe.visible &&= !document.hidden;
            },
            recordLongTasks: (entries) => {
              probe.longTasks.push(
                ...entries.map((entry) => ({
                  startTime: entry.startTime,
                  duration: entry.duration,
                })),
              );
            },
          };
          const frame = (time: number) => {
            if (last !== null && !document.hidden) {
              probe.frames.push({ startTime: last, duration: time - last });
            }
            last = document.hidden ? null : time;
            probe.animation = requestAnimationFrame(frame);
          };
          if (PerformanceObserver.supportedEntryTypes.includes('longtask')) {
            probe.observer = new PerformanceObserver((list) => {
              probe.recordLongTasks(list.getEntries());
            });
            probe.observer.observe({ type: 'longtask', buffered: true });
          }
          document.addEventListener('visibilitychange', probe.visibilityChanged);
          probe.animation = requestAnimationFrame(frame);
          scope.__readonlyStartupProbe = probe;
        });
        await $('[data-testid="dashboard-layout"]').waitForDisplayed({ timeout: 30_000 });
        const shellWallMs = performance.now() - started;
        const shellPageMs = await browser.execute(() => performance.now());
        const settings = await invokeInApp<AppSettings>('get_settings');
        expect(settings.active_game_id).toBe(READONLY_AUDIT_GAME);
        expect(settings.games.find((game) => game.id === READONLY_AUDIT_GAME)?.mod_path).toBe(
          audit.mods,
        );
        if (!settings.hotkeys) throw new Error('Native audit omitted hotkey configuration');
        expect(settings.hotkeys.enabled).toBe(false);
        await observeCoreReady();
        const coreWallMs = performance.now() - started;
        await browser.waitUntil(
          () =>
            browser.execute(
              () =>
                document.querySelector('main') !== null &&
                document.querySelector('main [role="status"][aria-busy="true"]') === null,
            ),
          {
            timeout: 180_000,
            interval: 100,
            timeoutMsg: 'Optional Home keybinding read did not settle',
          },
        );
        const optionalWallMs = performance.now() - started;
        const ui = await browser.execute(() => {
          const scope = window as unknown as { __readonlyStartupProbe: StartupProbe };
          const probe = scope.__readonlyStartupProbe;
          cancelAnimationFrame(probe.animation);
          if (probe.observer) probe.recordLongTasks(probe.observer.takeRecords());
          probe.observer?.disconnect();
          document.removeEventListener('visibilitychange', probe.visibilityChanged);
          return {
            frameGaps: probe.frames.map((event) => event.duration),
            longTasks: probe.longTasks.map((event) => event.duration),
            frameEvents: probe.frames,
            longTaskEvents: probe.longTasks,
            frameObservationStartMs: probe.startedPageMs,
            timeOrigin: performance.timeOrigin,
            pageVisible: probe.visible && !document.hidden,
            viewport: {
              width: window.innerWidth,
              height: window.innerHeight,
              devicePixelRatio: window.devicePixelRatio,
            },
            homeHasError: document.querySelector('main [role="alert"]') !== null,
          };
        });
        if (stopCpuProfile) await stopCpuProfile();
        expect(ui.homeHasError).toBe(false);
        const objects = await getObjects(READONLY_AUDIT_GAME);
        const objectCount = objects.length;
        const modCount = objects.reduce((total, object) => total + object.mod_count, 0);
        expect(modCount).toBeGreaterThan(0);
        expectedModCount ??= modCount;
        expectedObjectCount ??= objectCount;
        expect(modCount).toBe(expectedModCount);
        expect(objectCount).toBe(expectedObjectCount);
        if (release && run === 0) {
          const denied = await invokeInApp('write_mod_ini', {
            path: path.join(audit.output, 'denied-write.ini'),
            content: 'Read-only audit must reject this command',
          }).then(
            () => null,
            (error: unknown) => String(error),
          );
          expect(denied).not.toBeNull();
          expect(denied).toContain('Command blocked by read-only startup audit');
          await expect(fs.stat(path.join(audit.output, 'denied-write.ini'))).rejects.toMatchObject({
            code: 'ENOENT',
          });
        }
        const newLog = await fs.readFile(logPath, 'utf8');
        if (!newLog.startsWith(oldLog)) throw new Error('Native audit log rotated');
        const log = newLog.slice(oldLog.length);
        expect(log).toContain('Read-only native startup audit enabled');
        expect(log).not.toContain('[post_apply] Starting post-apply tasks');
        const registrations = log
          .split('\n')
          .filter((line) => line.includes('watcher registration session='));
        const baselines = log
          .split('\n')
          .filter(
            (line) =>
              line.includes(`disk reconcile timing game_id=${READONLY_AUDIT_GAME} `) &&
              line.includes('reason=StartupBoot'),
          );
        const switched = log
          .split('\n')
          .filter(
            (line) =>
              line.includes(`disk reconcile timing game_id=${READONLY_AUDIT_GAME} `) &&
              line.includes('reason=GameSwitched'),
          );
        expect(registrations).toHaveLength(1);
        expect(baselines).toHaveLength(1);
        expect(switched).toHaveLength(0);
        measurements.push({
          run,
          startedUnixMs,
          shellWallMs,
          shellPageMs,
          coreWallMs,
          optionalWallMs,
          ...ui,
          timing: baselines[0],
          registrations: registrations.length,
          sampleKind: run === 0 ? 'empty-index-process-start' : 'indexed-process-restart',
          buildMode: process.env.EMMM_E2E_BUILD_MODE ?? 'debug',
          cpuProfile: stopCpuProfile !== null,
          objectCount,
          modCount,
          modsPerCoreSecond: modCount / (coreWallMs / 1000),
          osCache: 'uncontrolled',
        });
        await fs.writeFile(
          path.join(audit.output, 'native-startup-measurements.json'),
          JSON.stringify(measurements, null, 2),
        );
        console.info(
          `readonly-gimi-startup ${JSON.stringify({ run, shellWallMs, shellPageMs, coreWallMs, optionalWallMs, longTaskCount: ui.longTasks.length, maxFrameGapMs: Math.max(0, ...ui.frameGaps), timing: baselines[0] })}`,
        );
      } catch (error) {
        measurementFailed = true;
        throw error;
      } finally {
        if (stopCpuProfile) {
          await stopCpuProfile().catch((cleanupError: unknown) => {
            if (!measurementFailed) throw cleanupError;
            console.error('Native CPU profile collection failed after startup error', cleanupError);
          });
        }
      }
    }
  });
});
