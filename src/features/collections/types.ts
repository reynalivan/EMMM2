import type { CollectionRuntimeSnapshot, CollectionSummary } from '../../types/collection';
import type { SafetyFilter } from '../../stores/appStore/explorerSlice';

export type CollectionSaveMode = 'save_current_state' | 'clone_snapshot';

export type CollectionSaveRequest = {
  mode: CollectionSaveMode;
  sourceCollectionId: string | null;
};

export type CollectionWorkspaceSource =
  { kind: 'current_runtime' } | { kind: 'stored_collection'; collectionId: string };

export type CollectionListRow =
  | {
      kind: 'current_runtime';
      rowId: string;
      label: string;
      modCount: number;
      isActive: boolean;
      isSafe: boolean;
      isSafetyClassified: boolean;
    }
  | {
      kind: 'stored_collection';
      rowId: string;
      collection: CollectionSummary;
    };

export const CURRENT_RUNTIME_ROW_ID = '__current_runtime__';

export function buildCollectionWorkspaceRows(
  collections: CollectionSummary[],
  runtime: CollectionRuntimeSnapshot | undefined,
  currentChangesLabel: string,
): CollectionListRow[] {
  const collectionRows: CollectionListRow[] = collections.map((collection) => ({
    kind: 'stored_collection',
    rowId: collection.id,
    collection,
  }));

  const hasLiveRuntimeContent =
    !!runtime && (runtime.current_mods.length > 0 || runtime.current_objects.length > 0);

  if (
    !runtime ||
    !runtime.is_dirty ||
    (runtime.runtime_status === 'unsaved' && !hasLiveRuntimeContent)
  ) {
    return collectionRows;
  }

  return [buildCurrentRuntimeRow(runtime, currentChangesLabel), ...collectionRows];
}

export function filterCollectionRowsBySafety(
  rows: CollectionListRow[],
  filter: SafetyFilter,
): CollectionListRow[] {
  if (filter === 'all') {
    return rows;
  }

  return rows.filter((row) => {
    const isSafe = row.kind === 'current_runtime' ? row.isSafe : row.collection.is_safe;
    const isFullyClassified =
      row.kind === 'current_runtime' ? row.isSafetyClassified : row.collection.is_safety_classified;

    return filter === 'safe' ? isFullyClassified && isSafe : !isSafe;
  });
}

export function isCollectionWorkspaceSourceEqual(
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

  if (right.kind === 'current_runtime') {
    return false;
  }

  return left.collectionId === right.collectionId;
}

export function buildCurrentRuntimeRow(
  snapshot: CollectionRuntimeSnapshot,
  label: string,
): CollectionListRow {
  return {
    kind: 'current_runtime',
    rowId: CURRENT_RUNTIME_ROW_ID,
    label,
    modCount: snapshot.projected_state.summary.active_root_count,
    isActive: true,
    isSafe: snapshot.is_safe,
    isSafetyClassified: snapshot.is_safety_classified,
  };
}
