import { invokeInApp } from './ipc.js';

/**
 * Read-back helpers for two-sided (disk + DB) assertions. All query the same
 * DB projection the UI renders, so asserting on them proves UI/DB sync without
 * relying on component testids that may not exist in the prod DOM.
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

/**
 * The command takes a single `ObjectFilter`, not flat args. `safe_mode` is
 * serde-skipped on the wire — the backend derives it from the active corridor,
 * so there is nothing for a caller to pass.
 */
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

/** Creates an object via the command (writes the folder to disk + a DB row). */
export async function createObject(
  gameId: string,
  name: string,
  objectType = 'Character',
): Promise<string> {
  return invokeInApp<string>('create_object_cmd', {
    input: { game_id: gameId, name, object_type: objectType },
  });
}

/**
 * Disk Reconcile: projects current filesystem reality into the DB (registers
 * newly discovered folders as `Other`, syncs enable/disable/rename/move). Run
 * this after creating mod folders on disk and before asserting on getObjects.
 */
export async function reconcile(gameId: string, reason = 'ManualRepair'): Promise<void> {
  await invokeInApp('reconcile_disk_state_cmd', { gameId, reason, forceFull: true });
}
