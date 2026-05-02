import type { CollectionSummary, CorridorSnapshot } from '../../types/collection';

export type CollectionSaveMode = 'save_current_state' | 'clone_snapshot';

export type CollectionSaveRequest = {
  mode: CollectionSaveMode;
  sourceCollectionId: string | null;
};

export type CollectionWorkspaceSource =
  | { kind: 'current_runtime' }
  | { kind: 'stored_collection'; collectionId: string };

export type CollectionListRow =
  | {
      kind: 'current_runtime';
      rowId: string;
      label: string;
      isSafe: boolean;
      modCount: number;
      isActive: boolean;
    }
  | {
      kind: 'stored_collection';
      rowId: string;
      collection: CollectionSummary;
    };

export const CURRENT_RUNTIME_ROW_ID = '__current_runtime__';

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
  snapshot: CorridorSnapshot,
  label: string,
): CollectionListRow {
  return {
    kind: 'current_runtime',
    rowId: CURRENT_RUNTIME_ROW_ID,
    label,
    isSafe: snapshot.is_safe,
    modCount: snapshot.projected_state.summary.active_root_count,
    isActive: true,
  };
}
