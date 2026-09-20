import type {
  WorkspaceExplorerQuery,
  WorkspaceExplorerSelection,
  WorkspaceExplorerSelectionInput,
} from '@/shared/api/tauri/bindings.gen';

export type WorkspaceExplorerSelectionModel =
  | { mode: 'explicit'; paths: ReadonlySet<string> }
  | {
      mode: 'all_matching';
      query: WorkspaceExplorerQuery;
      listingRevision: string;
      excludedPaths: ReadonlySet<string>;
      totalMatching: number;
    };

export const WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT = 10_000;

export function workspaceExplorerQueryScopeKey(query: WorkspaceExplorerQuery): string {
  return JSON.stringify([
    query.game_id,
    query.explorer_sub_path,
    query.search_query,
    query.sort_field,
    query.sort_order,
    query.safety_filter,
  ]);
}

export function workspaceExplorerSelectionCount(
  selection: WorkspaceExplorerSelectionModel,
): number {
  if (selection.mode === 'explicit') {
    return selection.paths.size;
  }

  return Math.max(0, selection.totalMatching - selection.excludedPaths.size);
}

export function assertWorkspaceExplorerBulkSelectionWithinLimit(
  selection: WorkspaceExplorerSelectionModel,
): void {
  if (workspaceExplorerSelectionCount(selection) > WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT) {
    throw new Error(
      `Bulk operations support at most ${WORKSPACE_EXPLORER_BULK_SELECTION_LIMIT.toLocaleString('en-US')} paths`,
    );
  }
}

export function isWorkspaceExplorerPathSelected(
  selection: WorkspaceExplorerSelectionModel,
  path: string,
): boolean {
  return selection.mode === 'explicit'
    ? selection.paths.has(path)
    : !selection.excludedPaths.has(path);
}

export function toWorkspaceExplorerSelectionInput(
  selection: WorkspaceExplorerSelectionModel,
  currentQuery: WorkspaceExplorerQuery,
  currentListingRevision: string,
): WorkspaceExplorerSelectionInput {
  assertWorkspaceExplorerBulkSelectionWithinLimit(selection);

  if (
    selection.mode === 'all_matching' &&
    workspaceExplorerQueryScopeKey(selection.query) !== workspaceExplorerQueryScopeKey(currentQuery)
  ) {
    throw new Error('Explorer selection is stale for the current query');
  }

  if (selection.mode === 'all_matching' && selection.listingRevision !== currentListingRevision) {
    throw new Error('Explorer selection is stale for the current listing revision');
  }

  const selectionInput: WorkspaceExplorerSelection =
    selection.mode === 'explicit'
      ? { mode: 'explicit', paths: Array.from(selection.paths) }
      : {
          mode: 'all_matching',
          excluded_paths: Array.from(selection.excludedPaths),
        };

  const input = {
    query: currentQuery,
    listing_revision: currentListingRevision,
    selection: selectionInput,
  };

  return input;
}
