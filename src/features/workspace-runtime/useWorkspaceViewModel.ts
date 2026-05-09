import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';
import { commands } from '../../lib/bindings';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useAppStore } from '../../stores/useAppStore';
import { toast } from '../../stores/useToastStore';
import { ItemStatus, type ObjectFilter } from '../../types/object';
import type { WorkspaceViewModel } from '../../types/workspace';
import {
  dispatchWorkspaceRuntimeEvent,
  useWorkspaceRuntimeSelector,
} from './state/workspaceStoreBridge';

export interface WorkspaceViewModelFilterInput {
  gameId: string | null;
  safeMode: boolean;
  selectedObjectType: string | null;
  objectMetaFilters: Record<string, string[]> | null;
  objectSortBy: 'name' | 'date' | 'rarity' | null;
  objectStatusFilter: 'all' | 'enabled' | 'disabled' | null;
}

export interface WorkspaceViewModelSelectionInput {
  selectedObjectFolderPath: string | null;
  explorerSubPath: string | undefined;
  selectedModPath: string | null;
}

interface UseWorkspaceViewModelOptions {
  filterOverrides?: Partial<WorkspaceViewModelFilterInput>;
}

export const workspaceKeys = {
  all: ['workspace', 'mods'] as const,
  viewModel: (
    filter: ObjectFilter,
    selectedObjectFolderPath: string | null,
    explorerSubPath: string | undefined,
    selectedModPath: string | null,
  ) =>
    [
      ...workspaceKeys.all,
      filter,
      selectedObjectFolderPath,
      explorerSubPath ?? null,
      selectedModPath,
    ] as const,
};

export function buildWorkspaceViewModelFilter(input: WorkspaceViewModelFilterInput): ObjectFilter {
  return {
    game_id: input.gameId ?? '',
    safe_mode: input.safeMode,
    object_type: input.selectedObjectType ?? null,
    search_query: null,
    meta_filters: input.objectMetaFilters,
    sort_by: input.objectSortBy,
    status_filter:
      input.objectStatusFilter === 'enabled'
        ? ItemStatus.Enabled
        : input.objectStatusFilter === 'disabled'
          ? ItemStatus.Disabled
          : null,
  };
}

export function buildWorkspaceViewModelInput(
  filter: ObjectFilter,
  selection: WorkspaceViewModelSelectionInput,
) {
  return {
    filter,
    selected_object_folder_path: selection.selectedObjectFolderPath,
    explorer_sub_path: selection.explorerSubPath ?? null,
    selected_mod_path: selection.selectedModPath,
  };
}

function buildReconciliationMessage(reason: string | null): string {
  if (reason === 'source_unavailable') {
    return 'Workspace source is unavailable. Selection was cleared.';
  }

  return 'Workspace target changed on disk. Selection was updated.';
}

export function useWorkspaceSelectionInput(): WorkspaceViewModelSelectionInput {
  const selectedObjectFolderPath = useWorkspaceRuntimeSelector(
    (state) => state.selectedObjectFolderPath,
  );
  const explorerSubPath = useWorkspaceRuntimeSelector((state) => state.explorerSubPath);
  const selectedModPath = useWorkspaceRuntimeSelector((state) => state.selectedModPath);

  return useMemo(
    () => ({
      selectedObjectFolderPath,
      explorerSubPath,
      selectedModPath,
    }),
    [selectedObjectFolderPath, explorerSubPath, selectedModPath],
  );
}

export function useWorkspaceViewModel(options?: UseWorkspaceViewModelOptions) {
  const { activeGame } = useActiveGame();
  const { safeMode, selectedObjectType, objectMetaFilters, objectSortBy, objectStatusFilter } =
    useAppStore();
  const selection = useWorkspaceSelectionInput();
  const filterInput = {
    gameId: options?.filterOverrides?.gameId ?? activeGame?.id ?? null,
    safeMode: options?.filterOverrides?.safeMode ?? safeMode,
    selectedObjectType: options?.filterOverrides?.selectedObjectType ?? selectedObjectType,
    objectMetaFilters: options?.filterOverrides?.objectMetaFilters ?? objectMetaFilters,
    objectSortBy: options?.filterOverrides?.objectSortBy ?? objectSortBy,
    objectStatusFilter: options?.filterOverrides?.objectStatusFilter ?? objectStatusFilter,
  };

  const filter = buildWorkspaceViewModelFilter(filterInput);

  const query = useQuery<WorkspaceViewModel>({
    // ObjectList and FolderGrid must read the same workspace snapshot.
    // Focus/navigation changes only reshape the query key; they must not trigger Disk Reconcile.
    queryKey: workspaceKeys.viewModel(
      filter,
      selection.selectedObjectFolderPath,
      selection.explorerSubPath,
      selection.selectedModPath,
    ),
    queryFn: () =>
      commands.getWorkspaceViewModel({
        input: buildWorkspaceViewModelInput(filter, selection),
      }),
    enabled: !!filterInput.gameId,
    staleTime: 30_000,
    refetchOnWindowFocus: false,
    placeholderData: keepPreviousData,
  });

  useEffect(() => {
    if (!query.data || query.isPlaceholderData) {
      return;
    }

    const reconciledSelection = query.data.selection;
    const nextExplorerSubPath = reconciledSelection.explorer_sub_path ?? undefined;
    const selectionMatches =
      selection.selectedObjectFolderPath === reconciledSelection.selected_object_folder_path &&
      selection.explorerSubPath === nextExplorerSubPath &&
      selection.selectedModPath === reconciledSelection.selected_mod_path;

    const reconciliationChanged = reconciledSelection.reconciliation_status !== 'unchanged';

    if (selectionMatches && !reconciliationChanged) {
      return;
    }

    dispatchWorkspaceRuntimeEvent({
      type: 'SELECTION_RECONCILED',
      selectedObjectFolderPath: reconciledSelection.selected_object_folder_path,
      explorerSubPath: nextExplorerSubPath,
      selectedModPath: reconciledSelection.selected_mod_path,
      currentPath: reconciledSelection.current_path,
      reconciliationStatus: reconciledSelection.reconciliation_status,
      reconciliationReason: reconciledSelection.reconciliation_reason,
      affectedPaths: reconciledSelection.affected_paths,
    });
    if (reconciliationChanged) {
      toast.info(buildReconciliationMessage(reconciledSelection.reconciliation_reason), 4000);
    }
  }, [
    query.data,
    query.isPlaceholderData,
    selection.explorerSubPath,
    selection.selectedModPath,
    selection.selectedObjectFolderPath,
  ]);

  return query;
}
