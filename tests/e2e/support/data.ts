import { browser } from '@wdio/globals';
import { invokeInApp } from './ipc.js';

export interface WorkspaceSwitchReceipt {
  disk_revision: number | null;
  source_epoch?: string | null;
}

export interface WorkspaceSwitchSnapshot {
  game_id: string;
  source_epoch: string;
  disk_revision: number;
  projected_revision: number;
  projection_repair_reason: string | null;
}

interface WorkspaceRuntimeSnapshot {
  runtime: {
    recovery_status: 'ready' | 'syncing' | 'failed';
    source_state: { status: 'available' | 'unavailable'; message: string | null };
  };
}

// The native watcher batches filesystem notifications for 500ms. Keep the
// readiness signal continuously true past that window so a delayed echo from a
// just-completed app mutation cannot race the next mutation.
const WATCHER_SETTLE_WINDOW_MS = 800;

/**
 * Read-back helpers for two-sided (disk + DB) assertions. All query the same
 * DB projection the UI consumes. These checks prove native read-model
 * convergence, not that React has rendered a particular frame.
 */

/** Subset of the app's ObjectSummary — only the fields the E2E asserts on. */
export interface ObjectSummary {
  id: string;
  name: string;
  folder_path: string;
  object_type: string;
  status: number | null; // ItemStatus: 0 = Disabled, 1 = Enabled
  mod_count: number;
  enabled_count: number;
}

/** The command takes a single `ObjectFilter`, not flat args. */
export async function getObjects(gameId: string): Promise<ObjectSummary[]> {
  const res = await invokeInApp<{ objects: ObjectSummary[]; lost_objects: string[] }>(
    'get_objects_cmd',
    {
      filter: {
        game_id: gameId,
        search_query: null,
        object_type: null,
        meta_filters: null,
        sort_by: null,
        status_filter: null,
      },
    },
  );
  return res.objects;
}

export async function findObject(gameId: string, name: string): Promise<ObjectSummary | undefined> {
  return (await getObjects(gameId)).find((o) => o.name === name);
}

function workspaceStructureInput(gameId: string): Record<string, unknown> {
  return {
    filter: {
      game_id: gameId,
      search_query: null,
      object_type: null,
      meta_filters: null,
      sort_by: null,
      status_filter: null,
    },
    selected_object_folder_path: null,
    explorer_sub_path: null,
  };
}

/** Waits for the existing activation/core-recovery snapshot, not optional indexing. */
export async function waitForWorkspaceCoreReady(gameId: string, timeout = 30_000): Promise<void> {
  let lastStatus = 'unavailable';
  let failure: string | null = null;
  let readySince: number | null = null;
  try {
    await browser.waitUntil(
      async () => {
        const snapshot = await invokeInApp<WorkspaceRuntimeSnapshot>('get_workspace_structure', {
          input: workspaceStructureInput(gameId),
        });
        lastStatus = `${snapshot.runtime.recovery_status}/${snapshot.runtime.source_state.status}`;
        if (snapshot.runtime.recovery_status === 'failed') {
          failure = `Workspace core recovery failed for ${gameId}`;
          return true;
        }
        const ready =
          snapshot.runtime.recovery_status === 'ready' &&
          snapshot.runtime.source_state.status === 'available';
        if (!ready) {
          readySince = null;
          return false;
        }
        readySince ??= Date.now();
        return Date.now() - readySince >= WATCHER_SETTLE_WINDOW_MS;
      },
      {
        timeout,
        interval: 100,
        timeoutMsg: `Workspace core readiness did not settle for ${gameId}`,
      },
    );
  } catch (error: unknown) {
    throw new Error(`Workspace core readiness did not settle for ${gameId}; last=${lastStatus}`, {
      cause: error,
    });
  }
  if (failure) {
    throw new Error(failure);
  }
}

/**
 * Waits for the disk receipt's own epoch/revision to reach the durable DB
 * checkpoint. A repair hole is a failure, never treated as eventual success.
 */
export async function waitForProjectionCheckpoint(
  gameId: string,
  receipt: WorkspaceSwitchReceipt,
  timeout = 30_000,
): Promise<WorkspaceSwitchSnapshot> {
  if (receipt.disk_revision === null || !receipt.source_epoch) {
    throw new Error('Switch receipt is missing its disk revision or source epoch');
  }
  const diskRevision = receipt.disk_revision;
  const sourceEpoch = receipt.source_epoch;

  let latest: WorkspaceSwitchSnapshot | null = null;
  let repairReason: string | null = null;
  try {
    await browser.waitUntil(
      async () => {
        latest = await invokeInApp<WorkspaceSwitchSnapshot>('get_workspace_switch_snapshot', {
          gameId,
        });
        if (latest.projection_repair_reason) {
          repairReason = latest.projection_repair_reason;
          return true;
        }
        return (
          latest.source_epoch === sourceEpoch &&
          latest.disk_revision >= diskRevision &&
          latest.projected_revision >= diskRevision
        );
      },
      {
        timeout,
        interval: 100,
        timeoutMsg: `Projection checkpoint did not reach disk revision ${diskRevision}`,
      },
    );
  } catch (error: unknown) {
    throw new Error(
      `Projection checkpoint did not reach disk revision ${diskRevision}; last=${JSON.stringify(latest)}`,
      { cause: error },
    );
  }

  if (repairReason) {
    throw new Error(`Projection requires repair: ${repairReason}`);
  }
  if (latest === null) {
    throw new Error('Projection checkpoint wait completed without a snapshot');
  }
  return latest;
}

/** Creates an object on disk and returns the reconciled projection id. */
export async function createObject(
  gameId: string,
  name: string,
  objectType = 'Character',
): Promise<string> {
  const result = await invokeInApp<{ id: string; sync_warning: unknown | null }>(
    'create_object_cmd',
    {
      input: { game_id: gameId, name, object_type: objectType },
    },
  );
  await waitForWorkspaceCoreReady(gameId);
  return result.id;
}

/**
 * Disk Reconcile: projects current filesystem reality into the DB (registers
 * newly discovered folders as `Other`, syncs enable/disable/rename/move). Run
 * this after creating mod folders on disk and before asserting on getObjects.
 */
export async function reconcile(gameId: string, reason = 'ManualRepair'): Promise<void> {
  await invokeInApp('reconcile_disk_state_cmd', { gameId, reason, forceFull: true });
  await waitForWorkspaceCoreReady(gameId);
}
