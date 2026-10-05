import { browser, $, $$, expect } from '@wdio/globals';
import fs from 'fs/promises';
import path from 'path';
import type {
  CollectionPreview,
  CollectionRuntimeSnapshot,
  CollectionSummary,
} from '../../../src/shared/api/tauri/bindings.gen.js';
import {
  addMockMod,
  createMockGame,
  listDir,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { gotoWorkspaceView, seedGamesAndOpenDashboard } from '../support/app.js';
import {
  findObject,
  waitForProjectionCheckpoint,
  waitForWorkspaceCoreReady,
  type WorkspaceSwitchReceipt,
  type WorkspaceSwitchSnapshot,
} from '../support/data.js';
import {
  armNextFrameProxy,
  invokeInApp,
  invokeManyInApp,
  waitForNextFrameProxy,
  type IpcRequest,
} from '../support/ipc.js';

const PROJECTION_TIMEOUT = 30_000;
const SYNTHETIC_BURST_SIZES = [100, 1_000] as const;

function emitResult(payload: Record<string, unknown>): void {
  console.log(`EMMM_E2E_RESULT ${JSON.stringify(payload)}`);
}

interface ExplorerNode {
  id: string;
  name: string;
  path: string;
  filesystem_identity?: string | null;
  is_enabled: boolean;
  is_effectively_active: boolean;
  switch_state: string;
}

interface ExplorerPage {
  items: ExplorerNode[];
}

interface WorkspaceSwitchResult extends WorkspaceSwitchReceipt {
  status: 'applied' | 'requires_duplicate_resolution' | 'requires_parent_enable' | 'noop';
  primary_path: string | null;
}

interface AppSettingsSnapshot {
  active_game_id: string | null;
}

function explorerPageArgs(gameId: string, subPath: string): Record<string, unknown> {
  return {
    input: {
      query: {
        game_id: gameId,
        explorer_sub_path: subPath,
        search_query: null,
        sort_field: 'name',
        sort_order: 'asc',
        safety_filter: 'all',
      },
      cursor: null,
      page_size: 100,
    },
  };
}

async function explorerNodes(gameId: string, subPath: string): Promise<ExplorerNode[]> {
  return (
    await invokeInApp<ExplorerPage>(
      'get_workspace_explorer_page',
      explorerPageArgs(gameId, subPath),
    )
  ).items;
}

async function waitForExplorerEntry(
  gameId: string,
  subPath: string,
  entryName: string,
): Promise<ExplorerNode> {
  let match: ExplorerNode | undefined;
  await browser.waitUntil(
    async () => {
      match = (await explorerNodes(gameId, subPath)).find(
        (node) => path.basename(node.path) === entryName,
      );
      return match !== undefined;
    },
    {
      timeout: PROJECTION_TIMEOUT,
      interval: 100,
      timeoutMsg: `Explorer did not publish ${subPath}/${entryName}`,
    },
  );
  if (!match) {
    throw new Error(`Explorer wait completed without ${subPath}/${entryName}`);
  }
  return match;
}

async function waitForDiskEntry(directory: string, entryName: string): Promise<void> {
  await browser.waitUntil(async () => (await listDir(directory)).includes(entryName), {
    timeout: PROJECTION_TIMEOUT,
    interval: 50,
    timeoutMsg: `Disk did not contain ${path.join(directory, entryName)}`,
  });
}

function switchRequest(
  gameId: string,
  target: ExplorerNode,
  desiredEnabled: boolean,
  intentRevision: number,
): IpcRequest {
  return {
    cmd: 'execute_workspace_switch',
    args: {
      input: {
        game_id: gameId,
        target: {
          kind: 'mod_path',
          value: target.path,
          expected_identity: target.filesystem_identity ?? null,
        },
        desired_enabled: desiredEnabled,
        resolution: 'normal',
        enable_disabled_ancestors: false,
        parent_enable_confirmation: null,
        origin_surface: 'folder_grid',
      },
      intentRevision,
    },
  };
}

async function executeSwitch(
  gameId: string,
  target: ExplorerNode,
  desiredEnabled: boolean,
  intentRevision: number,
): Promise<WorkspaceSwitchResult> {
  const request = switchRequest(gameId, target, desiredEnabled, intentRevision);
  return invokeInApp<WorkspaceSwitchResult>(request.cmd, request.args);
}

async function selectObjectByName(name: string): Promise<void> {
  const row = await $(`//*[@data-object-id and .//span[normalize-space(.)="${name}"]]`);
  await row.waitForClickable({ timeout: 8_000 });
  await row.click();
  await $('[data-testid="folder-grid"]').waitForExist({ timeout: 8_000 });
}

async function visibleElement(selector: string) {
  let visible: WebdriverIO.Element | undefined;
  await browser.waitUntil(
    async () => {
      for (const candidate of await $$(selector)) {
        if (await candidate.isDisplayed()) {
          visible = candidate;
          return true;
        }
      }
      return false;
    },
    { timeout: 5_000, interval: 50, timeoutMsg: `No visible element matches ${selector}` },
  );
  if (!visible) {
    throw new Error(`No visible element matches ${selector}`);
  }
  return visible;
}

async function selectGameByName(name: string): Promise<void> {
  const selector = await visibleElement('button[aria-label="Select Game"]');
  await selector.waitForClickable({ timeout: 5_000 });
  await selector.click();
  const option = await visibleElement(
    `//*[contains(concat(' ', normalize-space(@class), ' '), ' dropdown-content ')]//ul//button[.//span[normalize-space(.)="${name}"]]`,
  );
  await option.waitForClickable({ timeout: 5_000 });
  await option.click();
}

describe('Native switching stability — disk-first acceptance', () => {
  const games: MockGame[] = [];
  let gameIds: string[] = [];

  before(async () => {
    for (let index = 0; index < 5; index += 1) {
      games.push(await createMockGame(`SwitchingStability${index + 1}`));
    }

    await addMockMod(games[0], 'FirstObject', 'FirstToggle');
    await addMockMod(games[0], 'BurstObject', 'BurstToggle');
    await addMockMod(games[0], 'AncestorObject', 'Parent/AncestorChild');
    const driftPath = await addMockMod(games[0], 'DriftObject', 'DriftA');
    await fs.writeFile(path.join(driftPath, 'payload.sentinel'), 'drift-payload');

    const collisionEnabled = await addMockMod(games[0], 'CollisionA', 'Collision');
    const collisionDisabled = await addMockMod(games[0], 'CollisionA', 'DISABLED Collision');
    const independent = await addMockMod(games[0], 'CollisionB', 'Collision');
    await fs.writeFile(path.join(collisionEnabled, 'payload.sentinel'), 'collision-enabled');
    await fs.writeFile(path.join(collisionDisabled, 'payload.sentinel'), 'collision-disabled');
    await fs.writeFile(path.join(independent, 'payload.sentinel'), 'independent');

    gameIds = await seedGamesAndOpenDashboard(
      games.map((game, index) => ({ game, name: `Native ${String.fromCharCode(65 + index)}` })),
    );
  });

  after(async () => {
    for (const game of games) {
      scheduleMockGameRemoval(game);
    }
  });

  afterEach(async function () {
    if (this.currentTest?.state !== 'failed') return;
    const dom = await browser.execute(() => ({
      inputs: Array.from(document.querySelectorAll<HTMLInputElement>('input[type="checkbox"]'))
        .slice(0, 20)
        .map((input) => ({
          label: input.getAttribute('aria-label'),
          checked: input.checked,
          disabled: input.disabled,
          visible: input.getBoundingClientRect().width > 0,
        })),
      buttons: Array.from(document.querySelectorAll<HTMLButtonElement>('button[aria-label]'))
        .slice(0, 20)
        .map((button) => ({
          label: button.getAttribute('aria-label'),
          disabled: button.disabled,
          visible: button.getBoundingClientRect().width > 0,
        })),
      dialogs: Array.from(document.querySelectorAll('[role="dialog"], .modal-open'))
        .slice(0, 4)
        .map((dialog) => dialog.textContent?.slice(0, 250)),
      gameOptions: Array.from(
        document.querySelectorAll<HTMLButtonElement>('.dropdown-content ul button'),
      )
        .slice(0, 12)
        .map((button) => ({
          text: button.textContent?.slice(0, 100),
          visible: button.getBoundingClientRect().width > 0,
        })),
    }));
    console.log(
      `EMMM_E2E_FAILURE ${JSON.stringify({ scenario: this.currentTest.title, error: this.currentTest.err?.message, dom })}`,
    );
  });

  it('uses a trusted WebDriver click for the first toggle after first-game core readiness', async () => {
    await gotoWorkspaceView('mods');
    await selectObjectByName('FirstObject');

    const gridCell = await $('//*[@role="gridcell" and .//h3[@title="FirstToggle"]]');
    await gridCell.waitForExist({ timeout: 8_000 });
    const toggle = await gridCell.$('input.toggle[aria-label]');
    await toggle.waitForClickable({ timeout: 5_000 });
    const original = await waitForExplorerEntry(gameIds[0], 'FirstObject', 'FirstToggle');
    const saved = await invokeInApp<CollectionSummary>('create_collection', {
      gameId: gameIds[0],
      name: 'Native baseline',
      saveMode: 'save_current_state',
      sourceCollectionId: null,
    });

    const before = await invokeInApp<WorkspaceSwitchSnapshot>('get_workspace_switch_snapshot', {
      gameId: gameIds[0],
    });
    await armNextFrameProxy();
    const clickStarted = performance.now();
    await toggle.click();
    const nextFrameProxyMs = await waitForNextFrameProxy();
    await waitForDiskEntry(path.join(games[0].modsPath, 'FirstObject'), 'DISABLED FirstToggle');

    const observedDiskMs = performance.now() - clickStarted;
    expect(nextFrameProxyMs).toBeGreaterThanOrEqual(0);
    const snapshot = await invokeInApp<WorkspaceSwitchSnapshot>('get_workspace_switch_snapshot', {
      gameId: gameIds[0],
    });
    expect(snapshot.disk_revision).toBeGreaterThan(before.disk_revision);
    const receipt = { disk_revision: snapshot.disk_revision, source_epoch: snapshot.source_epoch };
    await waitForProjectionCheckpoint(gameIds[0], receipt);
    expect(await toggle.isEnabled()).toBe(true);
    expect(await toggle.isSelected()).toBe(false);
    expect((await findObject(gameIds[0], 'FirstObject'))?.enabled_count).toBe(0);
    const projected = await waitForExplorerEntry(gameIds[0], 'FirstObject', 'DISABLED FirstToggle');
    expect(projected.id).toBe(original.id);
    expect(original.filesystem_identity).toBeTruthy();
    expect(projected.filesystem_identity).toBe(original.filesystem_identity);
    expect(projected.is_enabled).toBe(false);
    expect(projected.is_effectively_active).toBe(false);
    const runtime = await invokeInApp<CollectionRuntimeSnapshot>('get_collection_runtime_state', {
      gameId: gameIds[0],
    });
    expect(
      runtime.current_mods.some(
        (mod) => path.basename(mod.mod_path).replace(/^DISABLED /, '') === 'FirstToggle',
      ),
    ).toBe(false);
    expect(
      runtime.projected_state.active_roots.some(
        (root) => path.basename(root.source_path).replace(/^DISABLED /, '') === 'FirstToggle',
      ),
    ).toBe(false);
    const history = await invokeInApp<CollectionPreview>('get_collection_preview', {
      collectionId: saved.id,
      gameId: gameIds[0],
    });
    expect(history.collection.mod_count).toBe(saved.mod_count);
    expect(
      history.projected_state.active_roots.some(
        (root) => path.basename(root.source_path) === 'FirstToggle',
      ),
    ).toBe(true);
    emitResult({
      scenario: 'trusted_first_toggle',
      input_kind: 'webdriver_pointer',
      next_frame_proxy_ms: nextFrameProxyMs,
      webdriver_to_disk_observed_ms: observedDiskMs,
      ipc_roundtrip_ms: null,
      disk_revision: receipt.disk_revision,
      source_epoch: receipt.source_epoch,
      actual_paint_measured: false,
    });
  });

  it('excludes an own-enabled child beneath a disabled ancestor without rewriting saved membership', async () => {
    const parent = await waitForExplorerEntry(gameIds[0], 'AncestorObject', 'Parent');
    const child = await waitForExplorerEntry(gameIds[0], 'AncestorObject/Parent', 'AncestorChild');
    expect(child.is_enabled).toBe(true);
    expect(child.is_effectively_active).toBe(true);
    const saved = await invokeInApp<CollectionSummary>('create_collection', {
      gameId: gameIds[0],
      name: 'Native ancestor baseline',
      saveMode: 'save_current_state',
      sourceCollectionId: null,
    });
    const receipt = await executeSwitch(gameIds[0], parent, false, 8_000);
    await waitForDiskEntry(path.join(games[0].modsPath, 'AncestorObject'), 'DISABLED Parent');
    await waitForProjectionCheckpoint(gameIds[0], receipt);
    const projected = await waitForExplorerEntry(
      gameIds[0],
      'AncestorObject/DISABLED Parent',
      'AncestorChild',
    );
    expect(projected.filesystem_identity).toBe(child.filesystem_identity);
    expect(projected.id).toBe(child.id);
    expect(projected.is_enabled).toBe(true);
    expect(projected.is_effectively_active).toBe(false);
    expect(projected.switch_state).toBe('blocked_by_ancestor');
    const current = await invokeInApp<CollectionRuntimeSnapshot>('get_collection_runtime_state', {
      gameId: gameIds[0],
    });
    expect(
      current.current_mods.some((mod) => path.basename(mod.mod_path) === 'AncestorChild'),
    ).toBe(false);
    expect(
      current.projected_state.active_roots.some(
        (root) => path.basename(root.source_path) === 'AncestorChild',
      ),
    ).toBe(false);
    const history = await invokeInApp<CollectionPreview>('get_collection_preview', {
      collectionId: saved.id,
      gameId: gameIds[0],
    });
    expect(history.collection.mod_count).toBe(saved.mod_count);
    expect(
      history.projected_state.active_roots.some(
        (root) => path.basename(root.source_path) === 'AncestorChild',
      ),
    ).toBe(true);
    emitResult({
      scenario: 'native_disabled_ancestor_consistency',
      child_own_enabled: true,
      child_effectively_active: projected.is_effectively_active,
      saved_membership_preserved: true,
      disk_revision: receipt.disk_revision,
      source_epoch: receipt.source_epoch,
    });
  });

  let intentRevision = 10_000;
  let expectedEnabled = true;
  for (const burstSize of SYNTHETIC_BURST_SIZES) {
    it(`accounts for ${burstSize} synthetic in-WebView intents separately from trusted pointer input`, async () => {
      const currentName = expectedEnabled ? 'BurstToggle' : 'DISABLED BurstToggle';
      const target = await waitForExplorerEntry(gameIds[0], 'BurstObject', currentName);
      const finalDesiredEnabled: boolean = !expectedEnabled;
      const requests = Array.from({ length: burstSize }, (_, index) => {
        const desiredEnabled =
          index === burstSize - 1
            ? finalDesiredEnabled
            : (index + Number(expectedEnabled)) % 2 === 0;
        intentRevision += 1;
        return switchRequest(gameIds[0], target, desiredEnabled, intentRevision);
      });

      const outcomes = await invokeManyInApp<WorkspaceSwitchResult>(requests);
      expect(outcomes).toHaveLength(burstSize);
      const fulfilledCount = outcomes.filter((outcome) => outcome.status === 'fulfilled').length;
      const rejectedCount = outcomes.filter((outcome) => outcome.status === 'rejected').length;
      expect(fulfilledCount).toBeGreaterThan(0);
      expect(fulfilledCount + rejectedCount).toBe(burstSize);
      expect(outcomes.every((outcome) => outcome.settled_at_ms >= outcome.started_at_ms)).toBe(
        true,
      );

      const latest = outcomes.at(-1);
      expect(latest?.status).toBe('fulfilled');
      if (latest?.status !== 'fulfilled' || !latest.value) {
        throw new Error(`Latest synthetic ${burstSize}-intent outcome did not succeed`);
      }

      const finalName = finalDesiredEnabled ? 'BurstToggle' : 'DISABLED BurstToggle';
      await waitForDiskEntry(path.join(games[0].modsPath, 'BurstObject'), finalName);
      const checkpointReceipt = outcomes
        .flatMap((outcome) =>
          outcome.status === 'fulfilled' && outcome.value ? [outcome.value] : [],
        )
        .filter((receipt) => receipt.disk_revision !== null && Boolean(receipt.source_epoch))
        .sort((left, right) => (right.disk_revision ?? -1) - (left.disk_revision ?? -1))[0];
      if (!checkpointReceipt) {
        throw new Error(`Synthetic ${burstSize}-intent burst produced no durable disk receipt`);
      }
      await waitForProjectionCheckpoint(gameIds[0], checkpointReceipt);
      emitResult({
        scenario: 'synthetic_same_target_burst',
        input_kind: 'synthetic_in_webview_ipc',
        sample_count: burstSize,
        fulfilled_count: fulfilledCount,
        rejected_count: rejectedCount,
        roundtrips_ms: outcomes.map((outcome) => outcome.settled_at_ms - outcome.started_at_ms),
        outcomes: outcomes.map((outcome) => outcome.status),
        latest_outcome: latest.status,
        checkpoint_disk_revision: checkpointReceipt.disk_revision,
        source_epoch: checkpointReceipt.source_epoch,
      });
      expectedEnabled = finalDesiredEnabled;
    });
  }

  it('follows repeated external renames, then checkpoints a switch at the current path', async () => {
    const objectDirectory = path.join(games[0].modsPath, 'DriftObject');
    await fs.rename(path.join(objectDirectory, 'DriftA'), path.join(objectDirectory, 'DriftB'));
    await waitForExplorerEntry(gameIds[0], 'DriftObject', 'DriftB');
    await fs.rename(path.join(objectDirectory, 'DriftB'), path.join(objectDirectory, 'DriftC'));
    const current = await waitForExplorerEntry(gameIds[0], 'DriftObject', 'DriftC');

    const receipt = await executeSwitch(gameIds[0], current, false, 20_001);
    expect(receipt.primary_path && path.basename(receipt.primary_path)).toBe('DISABLED DriftC');
    await waitForDiskEntry(objectDirectory, 'DISABLED DriftC');
    await waitForProjectionCheckpoint(gameIds[0], receipt);
    await waitForExplorerEntry(gameIds[0], 'DriftObject', 'DISABLED DriftC');
    expect(
      await fs.readFile(path.join(objectDirectory, 'DISABLED DriftC', 'payload.sentinel'), 'utf8'),
    ).toBe('drift-payload');
  });

  it('keeps the latest A-B-A activation and returns to the ready source', async () => {
    await selectGameByName('Native B');
    await selectGameByName('Native A');
    await browser.waitUntil(
      async () =>
        (await invokeInApp<AppSettingsSnapshot>('get_settings')).active_game_id === gameIds[0],
      { timeout: PROJECTION_TIMEOUT, interval: 50, timeoutMsg: 'A-B-A activation did not settle' },
    );
    await waitForWorkspaceCoreReady(gameIds[0]);
    expect((await invokeInApp<AppSettingsSnapshot>('get_settings')).active_game_id).toBe(
      gameIds[0],
    );
  });

  it('preserves true-collision payloads while equal names in another parent switch independently', async () => {
    const collisionDirectory = path.join(games[0].modsPath, 'CollisionA');
    const colliding = await waitForExplorerEntry(gameIds[0], 'CollisionA', 'Collision');
    let collisionRejected = false;
    try {
      await executeSwitch(gameIds[0], colliding, false, 30_001);
    } catch {
      collisionRejected = true;
    }
    expect(collisionRejected).toBe(true);
    expect(
      await fs.readFile(path.join(collisionDirectory, 'Collision', 'payload.sentinel'), 'utf8'),
    ).toBe('collision-enabled');
    expect(
      await fs.readFile(
        path.join(collisionDirectory, 'DISABLED Collision', 'payload.sentinel'),
        'utf8',
      ),
    ).toBe('collision-disabled');

    const independent = await waitForExplorerEntry(gameIds[0], 'CollisionB', 'Collision');
    const receipt = await executeSwitch(gameIds[0], independent, false, 30_002);
    await waitForProjectionCheckpoint(gameIds[0], receipt);
    expect(
      await fs.readFile(
        path.join(games[0].modsPath, 'CollisionB', 'DISABLED Collision', 'payload.sentinel'),
        'utf8',
      ),
    ).toBe('independent');
    expect(await listDir(collisionDirectory)).toEqual(
      expect.arrayContaining(['Collision', 'DISABLED Collision']),
    );
  });
});
