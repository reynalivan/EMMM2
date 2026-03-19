import { getCorridorStateName } from '../../../lib/corridorLabels';
import type { CollectionWorkspaceSource } from '../../../lib/corridorSelection';
import type { Collection, CorridorRuntimeSnapshot } from '../../../types/collection';

export type CollectionWorkspaceRowKind =
  | 'current_runtime'
  | 'stored_unsaved_snapshot'
  | 'named_collection';

export type CollectionWorkspacePrimaryAction = 'save_current' | 'save_snapshot' | 'apply';

export interface CollectionWorkspaceRow {
  collection: Collection;
  source: CollectionWorkspaceSource;
  sourceKind: CollectionWorkspaceRowKind;
  primaryActionKind: CollectionWorkspacePrimaryAction;
}

function buildCurrentRuntimeCollection(
  gameId: string,
  safeMode: boolean,
  memberCount: number,
  snapshot: CorridorRuntimeSnapshot,
): Collection {
  return {
    id: `__current_runtime__:${gameId}:${safeMode}`,
    name: getCorridorStateName(snapshot.state_name),
    game_id: gameId,
    is_safe_context: safeMode,
    member_count: memberCount,
    is_last_unsaved: false,
  };
}

function buildStoredCollectionRow(collection: Collection): CollectionWorkspaceRow {
  if (collection.is_last_unsaved) {
    return {
      collection,
      source: {
        kind: 'stored_collection',
        collection_id: collection.id,
      },
      sourceKind: 'stored_unsaved_snapshot',
      primaryActionKind: 'save_snapshot',
    };
  }

  return {
    collection,
    source: {
      kind: 'stored_collection',
      collection_id: collection.id,
    },
    sourceKind: 'named_collection',
    primaryActionKind: 'apply',
  };
}

export function buildCollectionWorkspaceRows(
  gameId: string | null | undefined,
  safeMode: boolean,
  collections: Collection[],
  runtimeSnapshot: CorridorRuntimeSnapshot | undefined,
): CollectionWorkspaceRow[] {
  const storedRows = collections.map(buildStoredCollectionRow);
  if (!gameId || runtimeSnapshot?.state_kind !== 'unsaved') {
    return storedRows;
  }

  const currentRuntimeRow: CollectionWorkspaceRow = {
    collection: buildCurrentRuntimeCollection(
      gameId,
      safeMode,
      runtimeSnapshot.roots.length,
      runtimeSnapshot,
    ),
    source: { kind: 'current_runtime' },
    sourceKind: 'current_runtime',
    primaryActionKind: 'save_current',
  };

  return [currentRuntimeRow, ...storedRows];
}

export function areWorkspaceSourcesEqual(
  left: CollectionWorkspaceSource | null,
  right: CollectionWorkspaceSource | null,
): boolean {
  if (!left || !right) {
    return left === right;
  }

  if (left.kind !== right.kind) {
    return false;
  }

  if (left.kind === 'current_runtime') {
    return true;
  }

  return left.collection_id === right.collection_id;
}

export function isWorkspaceSourceAvailable(
  rows: CollectionWorkspaceRow[],
  source: CollectionWorkspaceSource | null,
): boolean {
  if (!source) {
    return false;
  }

  return rows.some((row) => areWorkspaceSourcesEqual(row.source, source));
}

export function resolvePreferredWorkspaceSource(
  rows: CollectionWorkspaceRow[],
  persistedSource: CollectionWorkspaceSource | null,
  runtimeSnapshot: CorridorRuntimeSnapshot | undefined,
): CollectionWorkspaceSource | null {
  if (isWorkspaceSourceAvailable(rows, persistedSource)) {
    return persistedSource;
  }

  const activeCollectionId = runtimeSnapshot?.active_collection_id;
  if (activeCollectionId) {
    const activeRow = rows.find(
      (row) =>
        row.source.kind === 'stored_collection' &&
        row.source.collection_id === activeCollectionId &&
        row.sourceKind === 'named_collection',
    );
    if (activeRow) {
      return activeRow.source;
    }
  }

  const currentRuntimeRow = rows.find((row) => row.source.kind === 'current_runtime');
  if (currentRuntimeRow) {
    return currentRuntimeRow.source;
  }

  const storedUnsavedRow = rows.find((row) => row.sourceKind === 'stored_unsaved_snapshot');
  if (storedUnsavedRow) {
    return storedUnsavedRow.source;
  }

  const firstNamedRow = rows.find((row) => row.sourceKind === 'named_collection');
  if (firstNamedRow) {
    return firstNamedRow.source;
  }

  return rows[0]?.source ?? null;
}

export function findWorkspaceRow(
  rows: CollectionWorkspaceRow[],
  source: CollectionWorkspaceSource | null,
): CollectionWorkspaceRow | null {
  if (!source) {
    return null;
  }

  return rows.find((row) => areWorkspaceSourcesEqual(row.source, source)) ?? null;
}
