import { browser, expect } from '@wdio/globals';
import { randomUUID } from 'crypto';
import fs from 'fs/promises';
import path from 'path';
import {
  addMockMods,
  createMockGame,
  scheduleMockGameRemoval,
  type MockGame,
} from '../support/fixtures.js';
import { seedGamesAndOpenDashboard } from '../support/app.js';
import { waitForProjectionCheckpoint, type WorkspaceSwitchReceipt } from '../support/data.js';
import {
  clearAsyncIpcInApp,
  invokeInApp,
  invokeManyInApp,
  readAsyncIpcInApp,
  startAsyncIpcInApp,
  type AsyncIpcState,
  type IpcRequest,
} from '../support/ipc.js';

const CORPUS_SIZE = 10_000;
const CASE_SIZES = [100, 1_000, 10_000] as const;
const NATIVE_CASE_TIMEOUT = 15 * 60_000;

interface ExplorerNode {
  path: string;
  filesystem_identity?: string | null;
}

interface ExplorerPage {
  items: ExplorerNode[];
  next_cursor: string | null;
}

interface BulkResult extends WorkspaceSwitchReceipt {
  success: string[];
  failures: { path: string; error: unknown }[];
  cancelled: boolean;
  processed_count: number;
  unprocessed_count: number;
}

interface SwitchResult extends WorkspaceSwitchReceipt {
  primary_path: string | null;
}

interface CorpusEntry {
  relativePath: string;
  identity: string;
}

function emitResult(payload: Record<string, unknown>): void {
  console.log(`EMMM_E2E_RESULT ${JSON.stringify(payload)}`);
}

async function pathExists(candidate: string): Promise<boolean> {
  try {
    await fs.access(candidate);
    return true;
  } catch (error: unknown) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      return false;
    }
    throw error;
  }
}

async function loadExplorerNodes(gameId: string, subPath: string): Promise<ExplorerNode[]> {
  const items: ExplorerNode[] = [];
  let cursor: string | null = null;
  do {
    const page: ExplorerPage = await invokeInApp<ExplorerPage>('get_workspace_explorer_page', {
      input: {
        query: {
          game_id: gameId,
          explorer_sub_path: subPath,
          search_query: null,
          sort_field: 'name',
          sort_order: 'asc',
          safety_filter: 'all',
        },
        cursor,
        page_size: 200,
      },
    });
    items.push(...page.items);
    cursor = page.next_cursor;
  } while (cursor !== null);
  return items;
}

function corpusEntries(nodes: readonly ExplorerNode[], modsRoot: string): readonly CorpusEntry[] {
  return Object.freeze(
    nodes.map((node) => {
      if (!node.filesystem_identity) {
        throw new Error(`Explorer node is missing native identity: ${node.path}`);
      }
      const relativePath = path.relative(modsRoot, node.path);
      if (!relativePath || relativePath.startsWith('..') || path.isAbsolute(relativePath)) {
        throw new Error(`Explorer node escaped the owned Mods root: ${node.path}`);
      }
      return Object.freeze({ relativePath, identity: node.filesystem_identity });
    }),
  );
}

function bulkRequest(
  gameId: string,
  entries: readonly CorpusEntry[],
  enable: boolean,
  intentRevision: number,
): IpcRequest {
  return {
    cmd: 'bulk_toggle_mods',
    args: {
      gameId,
      paths: entries.map((entry) => entry.relativePath),
      enable,
      operationId: `native-bulk-${randomUUID()}`,
      intentRevision,
      expectedIdentities: entries.map((entry): [string, string] => [
        entry.relativePath,
        entry.identity,
      ]),
    },
  };
}

function disabledEntries(entries: readonly CorpusEntry[]): readonly CorpusEntry[] {
  return Object.freeze(
    entries.map((entry) => {
      const directory = path.dirname(entry.relativePath);
      const disabledName = `DISABLED ${path.basename(entry.relativePath)}`;
      return Object.freeze({
        relativePath: directory === '.' ? disabledName : path.join(directory, disabledName),
        identity: entry.identity,
      });
    }),
  );
}

function switchRequest(gameId: string, node: ExplorerNode, intentRevision: number): IpcRequest {
  return {
    cmd: 'execute_workspace_switch',
    args: {
      input: {
        game_id: gameId,
        target: {
          kind: 'mod_path',
          value: node.path,
          expected_identity: node.filesystem_identity ?? null,
        },
        desired_enabled: false,
        resolution: 'normal',
        enable_disabled_ancestors: false,
        parent_enable_confirmation: null,
        origin_surface: 'folder_grid',
      },
      intentRevision,
    },
  };
}

function requireFulfilled<T>(
  state: AsyncIpcState<T>,
  label: string,
): Extract<AsyncIpcState<T>, { status: 'fulfilled' }> {
  if (state.status === 'pending') {
    throw new Error(`${label} was still pending`);
  }
  if (state.status === 'rejected') {
    throw new Error(`${label} rejected: ${JSON.stringify(state.error)}`);
  }
  return state;
}

describe('Native switching bulk fairness and memory', function () {
  this.timeout(NATIVE_CASE_TIMEOUT);

  let game: MockGame;
  let gameId = '';
  let entries: readonly CorpusEntry[] = [];
  let leafNodes: ExplorerNode[] = [];
  let intentRevision = 40_000;

  before(async function () {
    this.timeout(NATIVE_CASE_TIMEOUT);
    game = await createMockGame('SwitchingBulk');
    const names = Array.from(
      { length: CORPUS_SIZE },
      (_, index) => `Mod${index.toString().padStart(5, '0')}`,
    );
    await addMockMods(game, 'BulkCorpus', names);
    await addMockMods(
      game,
      'HealthyLeaves',
      CASE_SIZES.map((size) => `Leaf${size}`),
    );
    await addMockMods(
      game,
      'OppositeBulk',
      Array.from({ length: 16 }, (_, index) => `Opposite${index.toString().padStart(2, '0')}`),
    );

    [gameId] = await seedGamesAndOpenDashboard([{ game }], NATIVE_CASE_TIMEOUT);
    const corpusNodes = await loadExplorerNodes(gameId, 'BulkCorpus');
    expect(corpusNodes).toHaveLength(CORPUS_SIZE);
    entries = corpusEntries(corpusNodes, game.modsPath);
    leafNodes = await loadExplorerNodes(gameId, 'HealthyLeaves');
    expect(leafNodes).toHaveLength(CASE_SIZES.length);
  });

  after(async function () {
    this.timeout(NATIVE_CASE_TIMEOUT);
    if (game) {
      scheduleMockGameRemoval(game);
    }
  });

  it('measures 100/1,000/10,000 actual bulk paths with an unrelated healthy leaf', async function () {
    this.timeout(NATIVE_CASE_TIMEOUT);

    for (const caseSize of CASE_SIZES) {
      const selected = Object.freeze(entries.slice(0, caseSize));
      const leaf = leafNodes.find((node) => path.basename(node.path) === `Leaf${caseSize}`);
      if (!leaf?.filesystem_identity) {
        throw new Error(`Healthy leaf ${caseSize} is missing its native identity`);
      }

      intentRevision += 2;
      const bulk = bulkRequest(gameId, selected, false, intentRevision);
      const leafRequest = switchRequest(gameId, leaf, intentRevision + 1);
      const bulkKey = `bulk-${caseSize}-${randomUUID()}`;
      const leafKey = `leaf-${caseSize}-${randomUUID()}`;
      const observerStartedAt = performance.now();
      await startAsyncIpcInApp(bulkKey, bulk.cmd, bulk.args);
      await startAsyncIpcInApp(leafKey, leafRequest.cmd, leafRequest.args);

      const firstDisabledPath = path.join(
        game.modsPath,
        'BulkCorpus',
        `DISABLED Mod${'0'.repeat(5)}`,
      );
      let firstDiskObservationMs: number | null = null;
      let peakJsHeapBytes: number | null = null;
      let leafState: AsyncIpcState<SwitchResult> = await readAsyncIpcInApp(leafKey);
      let bulkState: AsyncIpcState<BulkResult> = await readAsyncIpcInApp(bulkKey);

      await browser.waitUntil(
        async () => {
          bulkState = await readAsyncIpcInApp<BulkResult>(bulkKey);
          leafState = await readAsyncIpcInApp<SwitchResult>(leafKey);
          for (const heap of [bulkState.current_js_heap_bytes, leafState.current_js_heap_bytes]) {
            if (heap !== null) {
              peakJsHeapBytes = Math.max(peakJsHeapBytes ?? heap, heap);
            }
          }
          if (firstDiskObservationMs === null && (await pathExists(firstDisabledPath))) {
            firstDiskObservationMs = performance.now() - observerStartedAt;
          }
          return bulkState.status !== 'pending' && leafState.status !== 'pending';
        },
        {
          timeout: NATIVE_CASE_TIMEOUT,
          interval: 10,
          timeoutMsg: `Native bulk ${caseSize} or its unrelated leaf did not settle`,
        },
      );

      const bulkDone = requireFulfilled(bulkState, `bulk ${caseSize}`);
      const leafDone = requireFulfilled(leafState, `leaf ${caseSize}`);
      const leafSettledBeforeBulk = leafDone.settled_at_ms < bulkDone.settled_at_ms;
      expect(bulkDone.value.failures).toHaveLength(0);
      expect(bulkDone.value.cancelled).toBe(false);
      expect(bulkDone.value.processed_count).toBe(caseSize);
      expect(bulkDone.value.success).toHaveLength(caseSize);
      expect(firstDiskObservationMs).not.toBeNull();
      if (caseSize === CORPUS_SIZE) {
        expect(leafSettledBeforeBulk).toBe(true);
      }

      await waitForProjectionCheckpoint(gameId, bulkDone.value, NATIVE_CASE_TIMEOUT);
      await waitForProjectionCheckpoint(gameId, leafDone.value, NATIVE_CASE_TIMEOUT);
      expect(
        await pathExists(path.join(game.modsPath, 'HealthyLeaves', `DISABLED Leaf${caseSize}`)),
      ).toBe(true);

      emitResult({
        scenario: 'native_bulk_with_unrelated_leaf',
        corpus_size: caseSize,
        bulk_operation_id: bulk.args?.operationId,
        bulk_webview_roundtrip_ms: bulkDone.settled_at_ms - bulkDone.started_at_ms,
        first_disk_observation_host_ms: firstDiskObservationMs,
        leaf_webview_roundtrip_ms: leafDone.settled_at_ms - leafDone.started_at_ms,
        leaf_settled_before_bulk: leafSettledBeforeBulk,
        fulfilled_paths: bulkDone.value.success.length,
        failed_paths: bulkDone.value.failures.length,
        cancelled: bulkDone.value.cancelled,
        peak_webview_js_heap_bytes: peakJsHeapBytes,
        settled_webview_js_heap_bytes: bulkDone.settled_js_heap_bytes,
        memory_scope: 'webview_js_heap_not_process_rss',
        disk_revision: bulkDone.value.disk_revision,
        source_epoch: bulkDone.value.source_epoch,
      });

      await clearAsyncIpcInApp(bulkKey);
      await clearAsyncIpcInApp(leafKey);

      if (caseSize !== CORPUS_SIZE) {
        intentRevision += 1;
        const restore = bulkRequest(gameId, disabledEntries(selected), true, intentRevision);
        const restored = await invokeInApp<BulkResult>(restore.cmd, restore.args);
        expect(restored.failures).toHaveLength(0);
        expect(restored.success).toHaveLength(caseSize);
        await waitForProjectionCheckpoint(gameId, restored, NATIVE_CASE_TIMEOUT);
      }
    }
  });

  it('keeps the latest opposite bulk intent on an overlapping frozen selection', async function () {
    this.timeout(NATIVE_CASE_TIMEOUT);
    const nodes = await loadExplorerNodes(gameId, 'OppositeBulk');
    const overlap = corpusEntries(nodes, game.modsPath);
    expect(overlap).toHaveLength(16);

    intentRevision += 2;
    const disable = bulkRequest(gameId, overlap, false, intentRevision);
    const enable = bulkRequest(gameId, overlap, true, intentRevision + 1);
    const outcomes = await invokeManyInApp<BulkResult>([disable, enable]);
    expect(outcomes).toHaveLength(2);
    expect(outcomes[1].status).toBe('fulfilled');

    await browser.waitUntil(
      async () =>
        Promise.all(
          overlap.map((entry) => pathExists(path.join(game.modsPath, entry.relativePath))),
        ).then((present) => present.every(Boolean)),
      {
        timeout: NATIVE_CASE_TIMEOUT,
        interval: 20,
        timeoutMsg: 'Latest opposite bulk intent did not leave every target enabled',
      },
    );

    const receipts = outcomes.flatMap((outcome) =>
      outcome.status === 'fulfilled' &&
      outcome.value?.disk_revision !== null &&
      outcome.value?.source_epoch
        ? [outcome.value]
        : [],
    );
    if (receipts.length > 0) {
      const latestReceipt = receipts.sort(
        (left, right) => (right.disk_revision ?? -1) - (left.disk_revision ?? -1),
      )[0];
      await waitForProjectionCheckpoint(gameId, latestReceipt, NATIVE_CASE_TIMEOUT);
    }

    emitResult({
      scenario: 'native_overlapping_opposite_bulk',
      target_count: overlap.length,
      outcomes: outcomes.map((outcome) => outcome.status),
      roundtrips_ms: outcomes.map((outcome) => outcome.settled_at_ms - outcome.started_at_ms),
      latest_desired_enabled: true,
      final_enabled_count: overlap.length,
    });
  });
});
