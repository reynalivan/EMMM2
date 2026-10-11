import { browser, $, expect } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';
import { bootApp, seedGameAndOpenDashboard } from '../support/app.js';
import {
  addMockMods,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { getObjects, waitForWorkspaceCoreReady } from '../support/data.js';
import { invokeInApp, invokeManyInApp } from '../support/ipc.js';
import type {
  AppSettings,
  DiskReconcileResult,
  KeyViewerRuntimeDiagnostics,
  SaveSettingsResult,
} from '../../../src/shared/api/tauri/bindings.gen.js';

const OWNED_ENTRYPOINT = '; EMMM-Artifact: KeyViewer v1\nnamespace = EMMMv1';

interface RuntimeStatus {
  game_id: string;
  generation: number;
  phase: 'queued' | 'running' | 'succeeded' | 'needs_manual_reload' | 'failed';
}

interface RuntimeCaptureWindow extends Window {
  __TAURI__: {
    event: {
      listen: (
        event: string,
        listener: (event: { payload: RuntimeStatus }) => void,
      ) => Promise<() => void>;
    };
  };
  __EMMM_RUNTIME_EVENTS__?: RuntimeStatus[];
  __EMMM_RUNTIME_UNLISTEN__?: () => void;
}

describe('Native normal sync and KeyViewer traversal', () => {
  let game: MockGame;
  let gameId: string;
  let activeDuplicate: string;
  let secondaryDuplicate: string;
  let disabledDuplicate: string;
  let foreignEntrypoint: string;

  before(async () => {
    expect(await invokeInApp<string>('plugin:app|identifier')).toBe('com.reynalivan.emmm.e2e');
    await bootApp();
    const settings = await invokeInApp<AppSettings>('get_settings');
    if (!settings.keyviewer) throw new Error('Native settings omitted KeyViewer configuration');
    await invokeInApp('save_settings', {
      settings: { ...settings, keyviewer: { ...settings.keyviewer, enabled: true } },
    });
    game = await createMockGame('NormalSyncKeyViewer');
    await addMockMods(
      game,
      'LargeObject',
      Array.from({ length: 512 }, (_, index) => `Mod${index}`),
    );
    await addMockMods(game, 'LargeObject', ['DISABLED Nested/Hidden']);
    for (let offset = 0; offset < 512; offset += 32) {
      await Promise.all(
        Array.from({ length: Math.min(32, 512 - offset) }, (_, index) => offset + index)
          .filter((index) => index % 3 === 0)
          .map((index) =>
            fs.writeFile(
              path.join(game.modsPath, 'LargeObject', `Mod${index}`, 'info.json'),
              JSON.stringify({
                actual_name: `Custom ${index}`,
                is_safe: index % 2 === 0,
                description: 'Owned synthetic runtime metadata. '.repeat(128),
              }),
            ),
          ),
      );
    }
    await fs.writeFile(
      path.join(game.modsPath, 'LargeObject', 'Mod0', 'mod.ini'),
      '[TextureOverrideRuntime]\nhash = df65bb00\nvb0 = ResourceBody\n[KeyRuntime]\nkey = F1\n',
    );
    const secondaryRoot = path.join(game.root, 'SharedMods');
    activeDuplicate = path.join(game.modsPath, 'LargeObject', 'Mod0', 'KeyViewer.ini');
    secondaryDuplicate = path.join(secondaryRoot, '.hidden', 'KeyViewer.ini');
    disabledDuplicate = path.join(
      game.modsPath,
      'LargeObject',
      'DISABLED Nested',
      'Hidden',
      'KeyViewer.ini',
    );
    foreignEntrypoint = path.join(secondaryRoot, 'Foreign', 'KeyViewer.ini');
    for (const entrypoint of [
      activeDuplicate,
      secondaryDuplicate,
      disabledDuplicate,
      foreignEntrypoint,
    ]) {
      await fs.mkdir(path.dirname(entrypoint), { recursive: true });
      await fs.writeFile(
        entrypoint,
        entrypoint === foreignEntrypoint ? 'user-owned content' : OWNED_ENTRYPOINT,
      );
    }
    await fs.writeFile(
      path.join(game.root, 'd3dx.ini'),
      '[Include]\ninclude_recursive = Mods\ninclude_recursive = SharedMods\ninclude = Core/GIMI/main.ini\n',
    );
    gameId = await seedGameAndOpenDashboard(game);
  });

  after(() => {
    if (game) scheduleMockGameRemoval(game);
  });

  it('preserves full/scoped projections and migrates only enabled owned duplicate entrypoints', async () => {
    const full = await invokeInApp<DiskReconcileResult>('reconcile_disk_state_cmd', {
      gameId,
      reason: 'ManualRepair',
      changedPaths: [],
      forceFull: true,
    });
    expect(full.status).toBe('Applied');
    expect(full.scan_scope).toBe('Full');
    await waitForWorkspaceCoreReady(gameId);
    const before = (await getObjects(gameId)).find(
      (object) => object.folder_path === 'LargeObject',
    );
    expect(before?.mod_count).toBe(513);

    const scoped = await invokeInApp<DiskReconcileResult>('reconcile_disk_state_cmd', {
      gameId,
      reason: 'WatcherBatch',
      changedPaths: [path.join(game.modsPath, 'LargeObject', 'Mod0')],
      forceFull: false,
    });
    expect(scoped.status).toBe('Applied');
    expect(scoped.scan_scope).toBe('Scoped');
    await waitForWorkspaceCoreReady(gameId);
    const after = (await getObjects(gameId)).find((object) => object.folder_path === 'LargeObject');
    expect(after?.id).toBe(before?.id);
    expect(after?.mod_count).toBe(513);

    await browser.waitUntil(
      async () => {
        try {
          await fs.access(path.join(game.modsPath, '.emmm_data', 'KeyViewer.ini'));
          await fs.access(`${activeDuplicate}.emmm-backup`);
          await fs.access(`${secondaryDuplicate}.emmm-backup`);
          return true;
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code === 'ENOENT') return false;
          throw error;
        }
      },
      {
        timeout: 30_000,
        timeoutMsg: 'Native KeyViewer publication did not finish duplicate migration',
      },
    );
    expect(await fs.readFile(`${activeDuplicate}.emmm-backup`, 'utf8')).toBe(OWNED_ENTRYPOINT);
    expect(await fs.readFile(`${secondaryDuplicate}.emmm-backup`, 'utf8')).toBe(OWNED_ENTRYPOINT);
    expect(await fs.readFile(disabledDuplicate, 'utf8')).toBe(OWNED_ENTRYPOINT);
    expect(await fs.readFile(foreignEntrypoint, 'utf8')).toBe('user-owned content');
    await expect($('[data-testid="dashboard-layout"]')).toBeDisplayed();
  });

  it('awaits hotkey publication through the worker and preserves committed failure warnings', async () => {
    const core = path.join(game.root, 'Core', 'GIMI', 'main.ini');
    const coreBytes = await fs.readFile(core);
    const artifact = path.join(game.modsPath, '.emmm_data', 'KeyViewer.ini');
    const artifactBefore = await fs.readFile(artifact, 'utf8');
    let settings = await invokeInApp<AppSettings>('get_settings');
    if (!settings.hotkeys || !settings.keyviewer) throw new Error('Runtime settings are missing');
    try {
      await fs.writeFile(core, 'namespace = GIMIv8\n; owned fixture renderer is unavailable\n');
      const failed = await invokeInApp<SaveSettingsResult>('save_hotkey_configuration', {
        expectedRevision: settings.revision,
        hotkeys: { ...settings.hotkeys, enabled: false, toggle_overlay: 'F6' },
        keyviewer: settings.keyviewer,
      });
      expect(failed.settings.hotkeys?.toggle_overlay).toBe('F6');
      expect(failed.sync_warning?.kind).toBe('RuntimeSyncPending');
      expect(failed.sync_warning?.message).toContain('RendererUnsupported');
      expect(await fs.readFile(artifact, 'utf8')).toBe(artifactBefore);
    } finally {
      await fs.writeFile(core, coreBytes);
    }
    settings = await invokeInApp<AppSettings>('get_settings');
    if (!settings.hotkeys || !settings.keyviewer) throw new Error('Runtime settings are missing');
    const saved = await invokeInApp<SaveSettingsResult>('save_hotkey_configuration', {
      expectedRevision: settings.revision,
      hotkeys: { ...settings.hotkeys, enabled: false, toggle_overlay: 'F7' },
      keyviewer: settings.keyviewer,
    });
    expect(saved.settings.hotkeys?.toggle_overlay).toBe('F7');
    if (saved.sync_warning) {
      expect(saved.sync_warning.kind).toBe('ManualReloadRequired');
    } else {
      expect(saved.sync_warning).toBeNull();
      const diagnostics = await invokeInApp<KeyViewerRuntimeDiagnostics>(
        'get_keyviewer_runtime_diagnostics',
        { gameId },
      );
      console.info(`native-keyviewer-no-reload diagnostics=${JSON.stringify(diagnostics)}`);
      expect(diagnostics.reload).toBe('not_required');
      expect(['published', 'unchanged']).toContain(diagnostics.publication);
    }
    expect(await fs.readFile(artifact, 'utf8')).toContain('key = F7');
    await expect($('[data-testid="dashboard-layout"]')).toBeDisplayed();
  });

  it('settles an overlapping native runtime burst and keeps the winning settings artifact', async () => {
    const settings = await invokeInApp<AppSettings>('get_settings');
    if (!settings.hotkeys || !settings.keyviewer) throw new Error('Runtime settings are missing');
    const capture = await browser.executeAsync((done: (result: { error?: string }) => void) => {
      const scope = window as unknown as RuntimeCaptureWindow;
      scope.__EMMM_RUNTIME_UNLISTEN__?.();
      scope.__EMMM_RUNTIME_EVENTS__ = [];
      scope.__TAURI__.event
        .listen('runtime_sync:status', ({ payload }) => {
          scope.__EMMM_RUNTIME_EVENTS__?.push(payload);
        })
        .then(
          (unlisten) => {
            scope.__EMMM_RUNTIME_UNLISTEN__ = unlisten;
            done({});
          },
          (error: unknown) => done({ error: String(error) }),
        );
    });
    if (capture.error) throw new Error(`Runtime listener failed: ${capture.error}`);
    const outcomes = await invokeManyInApp<SaveSettingsResult | number>([
      { cmd: 'retry_runtime_sync', args: { gameId } },
      {
        cmd: 'save_hotkey_configuration',
        args: {
          expectedRevision: settings.revision,
          hotkeys: { ...settings.hotkeys, enabled: false, toggle_overlay: 'F8' },
          keyviewer: settings.keyviewer,
        },
      },
      ...Array.from({ length: 4 }, () => ({ cmd: 'retry_runtime_sync', args: { gameId } })),
    ]);
    expect(outcomes).toHaveLength(6);
    expect(outcomes.every((outcome) => outcome.status === 'fulfilled')).toBe(true);
    const saved = outcomes[1]?.value;
    if (!saved || typeof saved === 'number') throw new Error('Settings request omitted its result');
    expect(saved.settings.hotkeys?.toggle_overlay).toBe('F8');
    if (saved.sync_warning) {
      expect(['RuntimeSyncPending', 'ManualReloadRequired']).toContain(saved.sync_warning.kind);
      if (saved.sync_warning.kind === 'RuntimeSyncPending') {
        expect(saved.sync_warning.message).toContain('superseded');
      }
    }
    const newestGeneration = Math.max(
      ...outcomes.flatMap((outcome) => (typeof outcome.value === 'number' ? [outcome.value] : [])),
    );
    await browser.waitUntil(
      async () => {
        const events = await browser.execute(
          () => (window as unknown as RuntimeCaptureWindow).__EMMM_RUNTIME_EVENTS__ ?? [],
        );
        const terminal = events.find(
          (event) =>
            event.game_id === gameId &&
            event.generation >= newestGeneration &&
            ['succeeded', 'needs_manual_reload', 'failed'].includes(event.phase),
        );
        if (terminal?.phase === 'failed') throw new Error('Winning runtime generation failed');
        return terminal !== undefined;
      },
      { timeout: 30_000, timeoutMsg: 'Winning native runtime generation did not settle' },
    );
    await browser.execute(() => {
      const scope = window as unknown as RuntimeCaptureWindow;
      scope.__EMMM_RUNTIME_UNLISTEN__?.();
      delete scope.__EMMM_RUNTIME_UNLISTEN__;
      delete scope.__EMMM_RUNTIME_EVENTS__;
    });
    const artifact = path.join(game.modsPath, '.emmm_data', 'KeyViewer.ini');
    await browser.waitUntil(
      async () => (await fs.readFile(artifact, 'utf8')).includes('key = F8'),
      { timeout: 30_000, timeoutMsg: 'Winning runtime request did not publish F8' },
    );
    expect((await invokeInApp<AppSettings>('get_settings')).hotkeys?.toggle_overlay).toBe('F8');
    await expect($('[data-testid="dashboard-layout"]')).toBeDisplayed();
  });
});
