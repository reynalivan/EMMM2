import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { commands } from '../../lib/bindings';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useAppStore } from '../../stores/useAppStore';
import { ItemStatus, type ObjectFilter } from '../../types/object';
import type { WorkspaceViewModel } from '../../types/workspace';
import { useWorkspaceRuntimeSelector } from './state/workspaceStoreBridge';

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

  return useQuery<WorkspaceViewModel>({
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
}
