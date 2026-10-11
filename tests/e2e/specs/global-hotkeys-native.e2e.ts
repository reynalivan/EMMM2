import { browser, expect } from '@wdio/globals';
import { execFile, spawn, type ChildProcess } from 'node:child_process';
import { promisify } from 'node:util';
import { setTimeout as delay } from 'node:timers/promises';
import fs from 'node:fs/promises';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
import {
  addMockMod,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { gotoWorkspaceView, seedGameAndOpenDashboard } from '../support/app.js';
import { getObjects, waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp } from '../support/ipc.js';
import {
  assertForegroundGameIdentity,
  type ForegroundIdentity,
} from '../support/hotkeyIdentity.js';
import type {
  AppSettings,
  BulkResult,
  CollectionRuntimeDescriptor,
  CollectionSummary,
  SaveSettingsResult,
  HotkeyConfig,
  HotkeyNotice,
} from '../../../src/shared/api/tauri/bindings.gen.js';

const OBJECT = 'Hotkey QA';
const physicalKeyboard = process.env.EMMM_E2E_HOTKEY_PHYSICAL_INPUT === '1';
const INPUT_TIMEOUT = physicalKeyboard ? 300_000 : 180_000;
const SETTLE_TIMEOUT = 30_000;
const STATUS_READ_RETRIES = 2;
const STATUS_READ_RETRY_DELAY = 50;
const DIAGNOSTIC_TIMEOUT = 5_000;
const operatorDirectory = process.env.EMMM_E2E_HOTKEY_OPERATOR_DIR;
interface ForegroundGameSource {
  pid: number;
  executable: string;
  title: string;
}
const foregroundGame: ForegroundGameSource | null = process.env.EMMM_E2E_HOTKEY_FOREGROUND_GAME
  ? JSON.parse(process.env.EMMM_E2E_HOTKEY_FOREGROUND_GAME)
  : null;
const bindings = {
  safe_mode: foregroundGame ? 'Ctrl+Alt+Shift+F6' : 'Ctrl+Alt+Shift+F13',
  next_preset: foregroundGame ? 'Ctrl+Alt+Shift+F8' : 'Ctrl+Alt+Shift+F14',
  prev_preset: foregroundGame ? 'Ctrl+Alt+Shift+F9' : 'Ctrl+Alt+Shift+F15',
  toggle_overlay: foregroundGame ? 'Ctrl+Alt+Shift+F10' : 'Ctrl+Alt+Shift+F16',
};

interface ReceiverStatus {
  token: string;
  role: 'game' | 'other';
  pid: number;
  foregroundPid: number;
  keys: Array<{
    key: string;
    control: boolean;
    alt: boolean;
    shift: boolean;
    foregroundPid: number;
    at: string;
  }>;
  focus: Array<{ pid: number; at: string }>;
  at: string;
  foregroundIdentity?: ForegroundIdentity | null;
}

async function diagnostic<T>(read: () => Promise<T>) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    const value = await Promise.race([
      read(),
      new Promise<never>((_, reject) => {
        timer = setTimeout(
          () => reject(new Error('Diagnostic read timed out')),
          DIAGNOSTIC_TIMEOUT,
        );
      }),
    ]);
    return { ok: true as const, value };
  } catch (error) {
    return { ok: false as const, error: error instanceof Error ? error.message : String(error) };
  } finally {
    if (timer) clearTimeout(timer);
  }
}

interface ScenarioEvidence {
  name: string;
  requestToken: string;
  targetPid: number;
  receiver: ReceiverStatus;
  directories: string[];
  activeCollection: string | null;
  safeMode: boolean;
  inputSource: 'computer-use' | 'physical';
}

type NativeInputCompletion = {
  token: string;
  targetPid: number;
  key: string;
  physicalInput?: boolean;
} & ({ error: string } | { before: ReceiverStatus; after: ReceiverStatus });

// Only a real Computer Use operator can complete an input request. DOM keyboard
// events and invoking an action over IPC are not substitutes for OS hotkeys.
(operatorDirectory ? describe : describe.skip)(
  'Native OS global hotkeys across owned applications',
  function () {
    this.timeout(6 * (INPUT_TIMEOUT + SETTLE_TIMEOUT));
    let game: MockGame;
    let gameId: string;
    let first: CollectionSummary;
    let second: CollectionSummary;
    let third: CollectionSummary;
    const receivers = new Map<'game' | 'other', ChildProcess>();
    const receiverErrors = new Map<'game' | 'other', Error>();
    const receiverStderr = new Map<'game' | 'other', string>();
    const inputTimes = new Map<string, number>();
    const inputTargetPids = new Map<string, number>();
    const evidence: ScenarioEvidence[] = [];
    let directory = '';
    let physicalRunStopped = false;

    async function receiver(role: 'game' | 'other'): Promise<ReceiverStatus> {
      let contents: string;
      for (let attempt = 0; ; attempt += 1) {
        try {
          contents = await fs.readFile(path.join(game.root, `${role}.status.json`), 'utf8');
          break;
        } catch (error) {
          if (
            !(error instanceof Error) ||
            !('code' in error) ||
            error.code !== 'EBUSY' ||
            attempt >= STATUS_READ_RETRIES
          )
            throw error;
          await delay(STATUS_READ_RETRY_DELAY);
        }
      }
      const value: ReceiverStatus = JSON.parse(contents);
      if (
        value.token !== game.cleanupToken ||
        value.role !== role ||
        value.pid !== receivers.get(role)?.pid
      )
        throw new Error('Passive receiver ownership mismatch');
      return value;
    }

    async function configure(
      enabled: boolean,
      patch: Partial<HotkeyConfig> = {},
      keyviewerEnabled?: boolean,
    ): Promise<void> {
      const settings = await invokeInApp<AppSettings>('get_settings');
      if (!settings.hotkeys || !settings.keyviewer)
        throw new Error('Missing runtime control settings');
      const saved = await invokeInApp<SaveSettingsResult>('save_hotkey_configuration', {
        expectedRevision: settings.revision ?? 0,
        hotkeys: { ...settings.hotkeys, ...bindings, enabled, ...patch },
        keyviewer: {
          ...settings.keyviewer,
          ...(keyviewerEnabled === undefined ? {} : { enabled: keyviewerEnabled }),
        },
      });
      expect(saved.settings.hotkeys?.enabled).toBe(enabled);
      await waitForWorkspaceCoreReady(gameId);
    }

    async function setupToggle(folderPath: string, enable: boolean): Promise<void> {
      const operationId = randomUUID();
      const request = {
        gameId,
        paths: [folderPath],
        enable,
        operationId,
        intentRevision: 1,
        expectedIdentities: null,
      };
      let result: BulkResult;
      try {
        result = await invokeInApp<BulkResult>('bulk_toggle_mods', request);
      } catch (error) {
        await fs.writeFile(
          path.join(directory, `${operationId}.bulk.json`),
          JSON.stringify({ request, error: error instanceof Error ? error.message : error }),
        );
        throw error;
      }
      await fs.writeFile(
        path.join(directory, `${operationId}.bulk.json`),
        JSON.stringify({ request, result }),
      );
      if (
        result.cancelled ||
        result.failures.length !== 0 ||
        result.success.length !== 1 ||
        result.processed_count !== 1 ||
        result.unprocessed_count !== 0
      )
        throw new Error(`Fixture bulk toggle failed: ${JSON.stringify({ request, result })}`);
    }

    async function input(
      name: string,
      key: string,
      role: 'game' | 'other' = 'other',
    ): Promise<string> {
      if (foregroundGame) {
        const accessibleKeys: Readonly<Record<string, string>> = {
          F13: 'F6',
          F14: 'F8',
          F15: 'F9',
          F16: 'F10',
        };
        key = key.replace(/F1[3-6]$/, (functionKey) => accessibleKeys[functionKey]);
      }
      const target = receivers.get(role);
      if (!target?.pid || target.exitCode !== null || target.signalCode !== null)
        throw new Error(
          `Input target is not running: ${role}, pid=${target?.pid}, exit=${target?.exitCode}, signal=${target?.signalCode}, stderr=${receiverStderr.get(role) ?? ''}`,
        );
      const token = randomUUID();
      const targetPid = foregroundGame?.pid ?? target.pid;
      const requestedAt = Date.now();
      await fs.writeFile(
        path.join(directory, 'request.json'),
        JSON.stringify({
          token,
          name,
          key,
          role,
          targetPid,
          observerPid: target.pid,
          observerExecutable: path.join(game.root, `hotkey-${role}.exe`),
          executable: foregroundGame?.executable ?? path.join(game.root, `hotkey-${role}.exe`),
          title: foregroundGame?.title ?? `EMMM Hotkey ${role} ${game.cleanupToken}`,
          fixtureRoot: game.fixtureRoot,
          cleanupToken: game.cleanupToken,
          completion: path.join(directory, `${token}.complete.json`),
          requestedAt,
          minimizeApp: !foregroundGame && name === 'safe-mode-true',
          foregroundGameSource: foregroundGame !== null,
          physicalInput: physicalKeyboard,
          expiresAt: requestedAt + INPUT_TIMEOUT,
          cancelled: path.join(directory, `${token}.cancelled.json`),
        }),
      );
      console.info(`NATIVE_HOTKEY_REQUEST ${name} ${token}`);
      const receipt: { value?: NativeInputCompletion; error?: unknown } = {};
      try {
        await browser.waitUntil(
          async () => {
            try {
              receipt.value = JSON.parse(
                await fs.readFile(path.join(directory, `${token}.complete.json`), 'utf8'),
              );
            } catch (error) {
              if (error instanceof Error && 'code' in error && error.code === 'ENOENT')
                return false;
              receipt.error = error;
            }
            return true;
          },
          {
            timeout: INPUT_TIMEOUT,
            interval: 200,
            timeoutMsg: `Real OS input was not completed: ${name}`,
          },
        );
      } catch (error) {
        if (physicalKeyboard) physicalRunStopped = true;
        const cancellation = await diagnostic(() =>
          fs.writeFile(
            path.join(directory, `${token}.cancelled.json`),
            JSON.stringify({ token, expiredAt: Date.now(), reason: 'Input wait failed' }),
          ),
        );
        if (!cancellation.ok)
          console.error(`Input cancellation evidence failed: ${cancellation.error}`);
        throw error;
      }
      if (receipt.error)
        throw new Error('Invalid native completion file', { cause: receipt.error });
      const completed = receipt.value;
      if (
        !completed ||
        completed.token !== token ||
        completed.targetPid !== targetPid ||
        completed.key !== key ||
        (completed.physicalInput ?? false) !== physicalKeyboard
      )
        throw new Error('OS input completion does not match this request');
      if ('error' in completed) throw new Error(`Native operator failed: ${completed.error}`);
      for (const observation of [completed.before, completed.after]) {
        const observedAt = Date.parse(observation.at);
        if (
          observation.token !== game.cleanupToken ||
          observation.role !== role ||
          observation.pid !== target.pid ||
          observation.foregroundPid !== targetPid ||
          !Number.isFinite(observedAt) ||
          observedAt < requestedAt ||
          observedAt > Date.now() + 500 ||
          Date.now() - observedAt >
            (physicalKeyboard && observation === completed.before ? INPUT_TIMEOUT : 15_000)
        )
          throw new Error(
            'Input must have fresh, owned foreground observations before and after the key',
          );
      }
      if (Date.parse(completed.after.at) < Date.parse(completed.before.at))
        throw new Error('Native input observations are out of order');
      if (foregroundGame) {
        assertForegroundGameIdentity(foregroundGame, completed.before.foregroundIdentity);
        assertForegroundGameIdentity(
          foregroundGame,
          completed.after.foregroundIdentity,
          completed.before.foregroundIdentity,
        );
      }
      inputTimes.set(token, Date.parse(completed.before.at));
      inputTargetPids.set(token, targetPid);
      expect((await receiver(role)).foregroundPid).toBe(targetPid);
      if (!foregroundGame && name === 'safe-mode-true')
        expect(await invokeInApp<boolean>('plugin:window|is_minimized', { label: 'main' })).toBe(
          true,
        );
      return token;
    }

    async function record(
      name: string,
      requestToken: string,
      role: 'game' | 'other' = 'other',
    ): Promise<void> {
      const settings = await invokeInApp<AppSettings>('get_settings');
      const runtime = await invokeInApp<CollectionRuntimeDescriptor>(
        'get_collection_runtime_descriptor',
        { gameId },
      );
      const target = await receiver(role);
      const targetPid = inputTargetPids.get(requestToken);
      if (targetPid === undefined) throw new Error('Missing native input target');
      expect(target.foregroundPid).toBe(targetPid);
      const inputAt = inputTimes.get(requestToken);
      if (inputAt === undefined) throw new Error('Missing native input observation');
      expect(
        target.focus
          .filter((item) => Date.parse(item.at) >= inputAt)
          .every((item) => item.pid === targetPid),
      ).toBe(true);
      evidence.push({
        name,
        requestToken,
        targetPid,
        receiver: target,
        directories: (await fs.readdir(path.join(game.modsPath, OBJECT))).sort(),
        activeCollection: runtime.active_collection_id,
        safeMode: settings.safety.runtime_safe_mode_by_game?.[gameId] ?? false,
        inputSource: physicalKeyboard ? 'physical' : 'computer-use',
      });
      await fs.writeFile(
        path.join(directory, 'report.json'),
        JSON.stringify(
          {
            identifier: await invokeInApp<string>('plugin:app|identifier'),
            buildHash: process.env.EMMM_E2E_BUILD_SHA256,
            fixture: { root: game.root, token: game.cleanupToken },
            bindings,
            foregroundGame,
            scenarios: evidence,
            limitations: [
              'Selected Mods always synthetic; no actual 3DMigoto rendering asserted',
              ...(foregroundGame
                ? [
                    'Foreground game is input source only; focused-overlay and direct receiver disabled-key receipt remain uncovered in this mode',
                  ]
                : []),
            ],
          },
          null,
          2,
        ),
      );
    }

    async function waitForUnsafe(enabled: boolean, safeEnabled = true): Promise<void> {
      await browser.waitUntil(
        async () => {
          const entries = await fs.readdir(path.join(game.modsPath, OBJECT));
          return (
            entries.includes(enabled ? 'Unsafe' : 'DISABLED Unsafe') &&
            entries.includes(safeEnabled ? 'Safe' : 'DISABLED Safe')
          );
        },
        {
          timeout: SETTLE_TIMEOUT,
          interval: 100,
          timeoutMsg: 'OS hotkey disk state did not settle',
        },
      );
      await waitForWorkspaceCoreReady(gameId);
      const object = (await getObjects(gameId)).find((item) => item.name === OBJECT);
      expect(object?.enabled_count).toBe(Number(enabled) + Number(safeEnabled));
    }

    async function overlayReplayCount(): Promise<number> {
      const logs = await invokeInApp<string[]>('get_logs', { limit: 5000 });
      return logs.filter((line) =>
        line.includes('Hotkey ToggleOverlay replayed to the active game'),
      ).length;
    }

    async function stopReceiver(role: 'game' | 'other'): Promise<void> {
      const child = receivers.get(role);
      if (!child?.pid || child.exitCode !== null || child.signalCode !== null) return;
      await fs.writeFile(path.join(game.root, `${role}.stop`), game.cleanupToken);
      await browser.waitUntil(() => child.exitCode !== null || child.signalCode !== null, {
        timeout: 10_000,
        interval: 100,
        timeoutMsg: `Owned receiver shutdown unverified: ${role}`,
      });
      expect(child.exitCode).toBe(0);
    }

    before(async () => {
      if (!operatorDirectory || !path.isAbsolute(operatorDirectory))
        throw new Error('Absolute hotkey operator directory required');
      if (physicalKeyboard && !foregroundGame)
        throw new Error('Physical keyboard validation requires the explicit foreground game');
      if (
        foregroundGame &&
        (!Number.isInteger(foregroundGame.pid) ||
          foregroundGame.pid <= 0 ||
          !path.isAbsolute(foregroundGame.executable) ||
          foregroundGame.title !== 'Genshin Impact' ||
          path.basename(foregroundGame.executable).toLowerCase() !== 'genshinimpact.exe')
      )
        throw new Error(
          'Foreground source must identify the explicitly requested running Genshin window',
        );
      expect(await invokeInApp<string>('plugin:app|identifier')).toBe('com.reynalivan.emmm.e2e');
      directory = operatorDirectory;
      await fs.mkdir(directory);
      game = await createMockGame('OS_hotkeys');
      await fs.writeFile(path.join(directory, 'fixture.json'), JSON.stringify(game));
      for (const mod of ['Safe', 'Unsafe']) await addMockMod(game, OBJECT, mod);
      await fs.appendFile(
        path.join(game.root, 'd3dx.ini'),
        '\n[Hunting]\nreload_config = Ctrl+Alt+Shift+F17\n',
      );
      const executable = path.join(game.root, 'hotkey-game.exe');
      await promisify(execFile)(
        'C:/Windows/Microsoft.NET/Framework64/v4.0.30319/csc.exe',
        [
          '/nologo',
          '/warnaserror+',
          '/target:winexe',
          `/out:${executable}`,
          '/reference:System.Windows.Forms.dll',
          '/reference:System.Drawing.dll',
          '/reference:System.Web.Extensions.dll',
          path.resolve('tests/e2e/support/hotkey-input-window.cs'),
        ],
        { windowsHide: true },
      );
      await fs.copyFile(executable, path.join(game.root, 'hotkey-other.exe'));
      for (const role of ['game', 'other'] as const) {
        const child = spawn(
          path.join(game.root, `hotkey-${role}.exe`),
          [
            game.root,
            game.cleanupToken,
            role,
            ...(foregroundGame ? [String(foregroundGame.pid)] : []),
          ],
          { windowsHide: foregroundGame !== null },
        );
        receivers.set(role, child);
        child.stderr?.on('data', (data: Buffer) =>
          receiverStderr.set(
            role,
            `${receiverStderr.get(role) ?? ''}${data.toString()}`.slice(-4096),
          ),
        );
        child.once('exit', (code, signal) =>
          console.info(
            `OWNED_RECEIVER_EXIT ${role} ${child.pid} code=${code} signal=${signal} stderr=${receiverStderr.get(role) ?? ''}`,
          ),
        );
        child.once('error', (error) => receiverErrors.set(role, error));
        await browser.waitUntil(
          async () => {
            if (receiverErrors.has(role)) throw receiverErrors.get(role);
            if (child.exitCode !== null || child.signalCode !== null)
              throw new Error(`Passive receiver exited: ${role}`);
            try {
              return (await receiver(role)).pid === child.pid;
            } catch (error) {
              if (error instanceof Error && 'code' in error && error.code === 'ENOENT')
                return false;
              throw error;
            }
          },
          { timeout: SETTLE_TIMEOUT, interval: 100 },
        );
      }
      gameId = await seedGameAndOpenDashboard(game);
      const settings = await invokeInApp<AppSettings>('get_settings');
      await invokeInApp('save_settings', {
        settings: {
          ...settings,
          games: settings.games.map((item) =>
            item.id === gameId ? { ...item, game_exe: executable } : item,
          ),
        },
      });
      await waitForWorkspaceCoreReady(gameId);
      await invokeInApp('toggle_mod_safe', {
        gameId,
        folderPath: path.join(game.modsPath, OBJECT, 'Safe'),
        safe: true,
      });
      first = await invokeInApp<CollectionSummary>('create_collection', {
        gameId,
        name: 'A All',
        saveMode: 'save_current_state',
      });
      await setupToggle(path.join(game.modsPath, OBJECT, 'Unsafe'), false);
      await waitForUnsafe(false);
      second = await invokeInApp<CollectionSummary>('create_collection', {
        gameId,
        name: 'B Safe',
        saveMode: 'save_current_state',
      });
      for (const [mod, enable] of [
        ['Unsafe', true],
        ['Safe', false],
      ] as const) {
        const spelling = mod === 'Unsafe' ? 'DISABLED Unsafe' : 'Safe';
        await setupToggle(path.join(game.modsPath, OBJECT, spelling), enable);
        await waitForWorkspaceCoreReady(gameId);
      }
      await waitForUnsafe(true, false);
      third = await invokeInApp<CollectionSummary>('create_collection', {
        gameId,
        name: 'C Unsafe',
        saveMode: 'save_current_state',
      });
      await invokeInApp('apply_collection', {
        gameId,
        collectionId: first.id,
        ignoreMissing: true,
      });
      await waitForUnsafe(true);
      await configure(true);
    });

    after(async () => {
      let disableError: unknown;
      try {
        if (gameId) await configure(false);
      } catch (error) {
        disableError = error;
      }
      let stopped = true;
      for (const role of receivers.keys()) {
        try {
          await stopReceiver(role);
        } catch {
          stopped = false;
        }
      }
      if (game && stopped) scheduleMockGameRemoval(game);
      if (directory && game)
        await fs.writeFile(
          path.join(directory, 'receiver-exits.json'),
          JSON.stringify(
            await Promise.all(
              [...receivers].map(async ([role, child]) => ({
                role,
                pid: child.pid,
                exitCode: child.exitCode,
                signalCode: child.signalCode,
                stderr: receiverStderr.get(role) ?? '',
                status: await fs
                  .readFile(path.join(game.root, `${role}.status.json`), 'utf8')
                  .catch((error) => {
                    if (error instanceof Error && 'code' in error && error.code === 'ENOENT')
                      return null;
                    throw error;
                  }),
                closing: await fs
                  .readFile(path.join(game.root, `${role}.closed.json`), 'utf8')
                  .catch((error) => {
                    if (error instanceof Error && 'code' in error && error.code === 'ENOENT')
                      return null;
                    throw error;
                  }),
              })),
            ),
            null,
            2,
          ),
        );
      if (!stopped)
        throw new Error('Owned receiver shutdown unverified; fixture retained', {
          cause: disableError,
        });
      if (disableError) throw disableError;
    });

    beforeEach(function () {
      if (physicalKeyboard && physicalRunStopped) this.skip();
    });

    afterEach(async function () {
      if (!gameId || !directory) return;
      if (physicalKeyboard && this.currentTest?.state === 'failed') physicalRunStopped = true;
      const [logs, settings, directories] = await Promise.all([
        diagnostic(() => invokeInApp<string[]>('get_logs', { limit: 5000 })),
        diagnostic(async () => {
          const snapshot = await invokeInApp<AppSettings>('get_settings');
          return {
            enabled: snapshot.hotkeys?.enabled,
            safeMode: snapshot.safety.runtime_safe_mode_by_game?.[gameId] ?? false,
            activeGameId: snapshot.active_game_id,
          };
        }),
        diagnostic(() => fs.readdir(path.join(game.modsPath, OBJECT))),
      ]);
      const writes = [
        diagnostic(() =>
          fs.writeFile(
            path.join(directory, 'last-outcome.json'),
            JSON.stringify({
              case: this.currentTest?.title,
              state: this.currentTest?.state,
              settings,
              directories,
              logsError: logs.ok ? null : logs.error,
              physicalRunStopped,
            }),
          ),
        ),
      ];
      if (logs.ok)
        writes.push(
          diagnostic(() =>
            fs.writeFile(path.join(directory, 'private-app-log.txt'), logs.value.join('\n')),
          ),
        );
      for (const result of await Promise.all(writes)) {
        if (!result.ok) console.error(`Native diagnostic write failed: ${result.error}`);
      }
    });

    it('toggles Safe Mode on/off from another foreground application without stealing focus', async () => {
      for (const enabled of [true, false]) {
        const token = await input(`safe-mode-${enabled}`, 'Control_L+Alt_L+Shift_L+F13');
        await waitForUnsafe(!enabled);
        const settings = await invokeInApp<AppSettings>('get_settings');
        expect(settings.safety.runtime_safe_mode_by_game?.[gameId] ?? false).toBe(enabled);
        expect((await receiver('other')).keys.map((item) => item.key)).not.toContain('F17');
        await record(`safe-mode-${enabled}`, token);
      }
    });

    it('cycles next/previous and wraps while another application remains foreground', async () => {
      for (const [name, key, target] of [
        ['next', 'F14', second],
        ['previous', 'F15', first],
        ['next-again', 'F14', second],
        ['next-third', 'F14', third],
        ['wrap', 'F14', first],
        ['previous-wrap', 'F15', third],
      ] as const) {
        const token = await input(name, `Control_L+Alt_L+Shift_L+${key}`);
        await browser.waitUntil(
          async () =>
            (
              await invokeInApp<CollectionRuntimeDescriptor>('get_collection_runtime_descriptor', {
                gameId,
              })
            ).active_collection_id === target.id,
          { timeout: SETTLE_TIMEOUT, interval: 100, timeoutMsg: `Preset did not settle: ${name}` },
        );
        await waitForUnsafe(target.id !== second.id, target.id !== third.id);
        expect((await receiver('other')).keys.map((item) => item.key)).not.toContain('F17');
        await record(name, token);
      }
    });

    (foregroundGame ? it.skip : it)(
      'does not replay overlay into another app, replays into the focused game and restores registration',
      async () => {
        const before = await overlayReplayCount();
        const token = await input('overlay-background', 'Control_L+Alt_L+Shift_L+F16');
        const until = Date.now() + 1500;
        await browser.waitUntil(
          async () => {
            expect(await overlayReplayCount()).toBe(before);
            expect((await receiver('other')).keys.map((item) => item.key)).not.toContain('F16');
            return Date.now() >= until;
          },
          { timeout: SETTLE_TIMEOUT, interval: 100 },
        );
        await record('overlay-background', token);
        for (let attempt = 1; attempt <= 5; attempt += 1) {
          const inputToken = await input(
            `overlay-focused-${attempt}`,
            'Control_L+Alt_L+Shift_L+F16',
            'game',
          );
          await browser.waitUntil(
            async () => {
              const received = (await receiver('game')).keys.filter((item) => item.key === 'F16');
              return (
                (await overlayReplayCount()) === before + attempt &&
                received.length === attempt &&
                received.every(
                  (item) =>
                    item.control &&
                    item.alt &&
                    item.shift &&
                    item.foregroundPid === receivers.get('game')?.pid,
                )
              );
            },
            {
              timeout: SETTLE_TIMEOUT,
              interval: 100,
              timeoutMsg: 'Focused overlay replay/registration restoration failed',
            },
          );
          const overlayKeys = (await receiver('game')).keys.filter((item) => item.key === 'F16');
          expect(overlayKeys.length).toBe(attempt);
          expect(
            overlayKeys.every(
              (item) =>
                item.control &&
                item.alt &&
                item.shift &&
                item.foregroundPid === receivers.get('game')?.pid,
            ),
          ).toBe(true);
          await record(`overlay-focused-${attempt}`, inputToken, 'game');
        }
      },
    );

    (foregroundGame ? it.skip : it)(
      'shows hotkey notifications on Home and Settings with independent native overlay controls',
      async () => {
        const nativeVisible = async () => {
          const labels = await invokeInApp<string[]>('plugin:window|get_all_windows');
          return (
            labels.includes('hotkey-notice') &&
            invokeInApp<boolean>('plugin:window|is_visible', { label: 'hotkey-notice' })
          );
        };
        const waitForNotice = async (kind: HotkeyNotice['kind']) => {
          const result: { notice: HotkeyNotice | null } = { notice: null };
          await browser.waitUntil(
            async () => {
              result.notice = await invokeInApp<HotkeyNotice | null>('get_hotkey_notice');
              return result.notice?.kind === kind;
            },
            {
              timeout: SETTLE_TIMEOUT,
              interval: 50,
              timeoutMsg: 'Committed hotkey feedback was not published',
            },
          );
          if (!result.notice) throw new Error('Notice wait completed without payload');
          return result.notice;
        };
        try {
          await invokeInApp('apply_collection', {
            gameId,
            collectionId: first.id,
            ignoreMissing: true,
          });
          await waitForUnsafe(true);
          await configure(
            true,
            { game_notification_overlay_enabled: true, preset_status_overlay_enabled: true },
            false,
          );
          await gotoWorkspaceView('dashboard');
          const otherToken = await input('notice-home-safe-on', 'Control_L+Alt_L+Shift_L+F13');
          const safeNotice = await waitForNotice('safe_mode_changed');
          expect(safeNotice.safe_mode).toBe(true);
          const host = await browser.$('[data-testid="hotkey-notice-host"]');
          await host.waitForExist({ timeout: 2000 });
          expect(await host.getText()).toContain('Safe Mode ON');
          expect(await host.getText()).toContain('Hotkey helpers');
          expect(await host.getText()).toContain(bindings.safe_mode);
          expect(await nativeVisible()).toBe(false);
          await waitForUnsafe(false);
          await record('notice-home-safe-on', otherToken);
          await host.waitForExist({ reverse: true, timeout: 7000 });

          await gotoWorkspaceView('settings');
          const gameToken = await input(
            'notice-game-safe-off',
            'Control_L+Alt_L+Shift_L+F13',
            'game',
          );
          const gameNotice = await waitForNotice('safe_mode_changed');
          expect(gameNotice.safe_mode).toBe(false);
          await browser.waitUntil(nativeVisible, {
            timeout: 3000,
            interval: 50,
            timeoutMsg: 'Opted-in game notification did not become visible',
          });
          expect(
            await invokeInApp<boolean>('plugin:window|is_always_on_top', {
              label: 'hotkey-notice',
            }),
          ).toBe(true);
          expect(
            await invokeInApp<boolean>('plugin:window|is_decorated', { label: 'hotkey-notice' }),
          ).toBe(false);
          expect((await receiver('game')).foregroundPid).toBe(receivers.get('game')?.pid);
          expect(await host.getText()).toContain('Safe Mode OFF');
          await waitForUnsafe(true);
          await record('notice-game-safe-off', gameToken, 'game');
          await browser.waitUntil(async () => !(await nativeVisible()), {
            timeout: 7000,
            interval: 50,
          });
          expect(await invokeInApp<HotkeyNotice | null>('get_hotkey_notice')).toBeNull();

          await configure(
            true,
            { game_notification_overlay_enabled: false, preset_status_overlay_enabled: false },
            true,
          );
          const nextToken = await input(
            'notice-game-next-muted',
            'Control_L+Alt_L+Shift_L+F14',
            'game',
          );
          const nextNotice = await waitForNotice('collection_applied');
          expect(nextNotice.previous_collection).toBe(first.name);
          expect(nextNotice.collection).toBe(second.name);
          expect(await host.getText()).toContain(second.name);
          expect(await host.getText()).toContain(`From ${first.name}`);
          expect(await host.getText()).toContain(bindings.prev_preset);
          expect(await host.getText()).toContain('Ctrl+Alt+Shift+F17');
          expect(await nativeVisible()).toBe(false);
          await waitForUnsafe(false);
          await record('notice-game-next-muted', nextToken, 'game');
          await fs.writeFile(
            path.join(directory, 'notice-evidence.json'),
            JSON.stringify(
              {
                identifier: await invokeInApp<string>('plugin:app|identifier'),
                buildHash: process.env.EMMM_E2E_BUILD_SHA256,
                notices: [safeNotice, gameNotice, nextNotice],
                nativeVisibility: { background: false, gameOptIn: true, gameOptOut: false },
                limitations: [
                  'Synthetic game/window/library only; actual exclusive-fullscreen rendering and mouse pass-through remain separate checks',
                ],
              },
              null,
              2,
            ),
          );
        } finally {
          await configure(
            true,
            { game_notification_overlay_enabled: false, preset_status_overlay_enabled: false },
            true,
          );
        }
      },
    );

    it('cycles presets even when the configured game process has stopped', async () => {
      await stopReceiver('game');
      const token = await input('game-stopped-next', 'Control_L+Alt_L+Shift_L+F14');
      await browser.waitUntil(
        async () =>
          (
            await invokeInApp<CollectionRuntimeDescriptor>('get_collection_runtime_descriptor', {
              gameId,
            })
          ).active_collection_id === first.id,
        { timeout: SETTLE_TIMEOUT, interval: 100 },
      );
      await waitForUnsafe(true);
      expect(receivers.get('game')?.exitCode).toBe(0);
      await record('game-stopped-next', token);
    });

    (foregroundGame ? it.skip : it)('stops dispatching when hotkeys are disabled', async () => {
      await configure(false);
      const receivedBefore = (await receiver('other')).keys.filter(
        (item) => item.key === 'F13',
      ).length;
      const directoriesBefore = await fs.readdir(path.join(game.modsPath, OBJECT));
      const token = await input('disabled-safe-mode', 'Control_L+Alt_L+Shift_L+F13');
      await browser.waitUntil(
        async () =>
          (await receiver('other')).keys.filter((item) => item.key === 'F13').length ===
          receivedBefore + 1,
        {
          timeout: SETTLE_TIMEOUT,
          interval: 100,
          timeoutMsg: 'Disabled test did not receive the physical OS chord',
        },
      );
      const until = Date.now() + 1500;
      await browser.waitUntil(
        async () => {
          const settings = await invokeInApp<AppSettings>('get_settings');
          expect(settings.safety.runtime_safe_mode_by_game?.[gameId] ?? false).toBe(false);
          expect((await fs.readdir(path.join(game.modsPath, OBJECT))).sort()).toEqual(
            directoriesBefore.sort(),
          );
          return Date.now() >= until;
        },
        { timeout: SETTLE_TIMEOUT, interval: 100 },
      );
      await record('disabled-safe-mode', token);
    });

    it('keeps bulk outcomes and disk projection consistent after repeated snapshot captures', async () => {
      await configure(false);
      await invokeInApp('apply_collection', {
        gameId,
        collectionId: first.id,
        ignoreMissing: true,
      });
      await waitForUnsafe(true);
      for (let iteration = 0; iteration < 8; iteration += 1) {
        const captured = await invokeInApp<CollectionSummary>('create_collection', {
          gameId,
          name: `Bulk regression ${iteration}`,
          saveMode: 'save_current_state',
        });
        for (const enable of [false, true]) {
          await setupToggle(
            path.join(game.modsPath, OBJECT, enable ? 'DISABLED Unsafe' : 'Unsafe'),
            enable,
          );
          await waitForUnsafe(enable);
        }
        await invokeInApp('delete_collection', { id: captured.id });
      }
      await fs.writeFile(
        path.join(directory, 'bulk-capture-regression.json'),
        JSON.stringify({
          captures: 8,
          toggles: 16,
          directories: await fs.readdir(path.join(game.modsPath, OBJECT)),
        }),
      );
    });
  },
);
