import { useShallow } from 'zustand/react/shallow';
import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';
import { commands } from '../../../shared/api/tauri/bindings';
import { useActiveGame } from '@/entities/game';
import { useAppStore } from '@/app/store';
import { ItemStatus, type ObjectFilter } from '@/entities/game-object';
import type {
  WorkspacePreviewContextStatus,
  WorkspacePreviewRequestIdentity,
  WorkspacePreviewResult,
  WorkspacePreviewSelection,
  WorkspaceSelection,
  WorkspaceStructureViewModel,
} from '@/entities/workspace';
import {
  dispatchWorkspaceRuntimeEvent,
  useWorkspaceRuntimeSelector,
} from '../state/workspaceStoreBridge';
import {
  buildSelectionReconciledEvent,
  shouldApplySelectionReconciledEvent,
  shouldRunSelectionReconciliationEffect,
  type WorkspaceViewModelSelectionInput,
} from '../utils/selectionReconciliation';
import { normalizeWorkspacePath } from '../utils/pathRewrite';

export interface WorkspaceViewModelFilterInput {
  gameId: string | null;
  selectedObjectType: string | null;
  objectMetaFilters: Record<string, string[]> | null;
  objectSortBy: 'name' | 'date' | 'rarity' | null;
  objectStatusFilter: 'all' | 'enabled' | 'disabled' | null;
}

interface UseWorkspaceStructureOptions {
  filterOverrides?: Partial<WorkspaceViewModelFilterInput>;
  selectionOverrides?: Partial<WorkspaceViewModelSelectionInput>;
  enabled?: boolean;
}

interface UseWorkspacePreviewOptions {
  enabled?: boolean;
}

interface WorkspaceStructureSelectionInput {
  selectedObjectFolderPath: string | null;
  explorerSubPath: string | undefined;
}

export interface WorkspacePreviewRequestInput {
  gameId: string | null;
  explorerSubPath: string | undefined;
  selectedModPath: string | null;
}

export const workspaceKeys = {
  all: ['workspace', 'mods'] as const,
  structures: ['workspace', 'mods', 'structure'] as const,
  previews: ['workspace', 'mods', 'preview'] as const,
  explorerPages: (query: import('@/entities/workspace').WorkspaceExplorerQuery | null) =>
    [...workspaceKeys.all, 'explorer-page', query] as const,
  structure: (
    filter: ObjectFilter,
    selectedObjectFolderPath: string | null,
    explorerSubPath: string | undefined,
  ) =>
    [
      ...workspaceKeys.structures,
      filter,
      selectedObjectFolderPath,
      explorerSubPath ?? null,
    ] as const,
  preview: (gameId: string, explorerSubPath: string | undefined, selectedModPath: string) =>
    [...workspaceKeys.previews, gameId, explorerSubPath ?? null, selectedModPath] as const,
};

export function buildWorkspaceViewModelFilter(input: WorkspaceViewModelFilterInput): ObjectFilter {
  return {
    game_id: input.gameId ?? '',
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

export function buildWorkspaceStructureInput(
  filter: ObjectFilter,
  selection: WorkspaceStructureSelectionInput,
) {
  return {
    filter,
    selected_object_folder_path: selection.selectedObjectFolderPath,
    explorer_sub_path: selection.explorerSubPath ?? null,
  };
}

export function buildWorkspacePreviewInput(input: WorkspacePreviewRequestInput) {
  return {
    game_id: input.gameId ?? '',
    explorer_sub_path: input.explorerSubPath ?? null,
    selected_mod_path: input.selectedModPath,
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

function useWorkspaceFilterInput(options?: UseWorkspaceStructureOptions) {
  const { activeGame } = useActiveGame();
  // Shared by ObjectList and FolderGrid — a bare useAppStore() here re-runs
  // both panes' filter build on any write to any slice.
  const { selectedObjectType, objectMetaFilters, objectSortBy, objectStatusFilter } = useAppStore(
    useShallow((state) => ({
      selectedObjectType: state.selectedObjectType,
      objectMetaFilters: state.objectMetaFilters,
      objectSortBy: state.objectSortBy,
      objectStatusFilter: state.objectStatusFilter,
    })),
  );

  return {
    gameId: options?.filterOverrides?.gameId ?? activeGame?.id ?? null,
    selectedObjectType: options?.filterOverrides?.selectedObjectType ?? selectedObjectType,
    objectMetaFilters: options?.filterOverrides?.objectMetaFilters ?? objectMetaFilters,
    objectSortBy: options?.filterOverrides?.objectSortBy ?? objectSortBy,
    objectStatusFilter: options?.filterOverrides?.objectStatusFilter ?? objectStatusFilter,
  };
}

function structureSelectionForRuntimeEvent(
  currentSelection: WorkspaceViewModelSelectionInput,
  structure: WorkspaceStructureViewModel,
): WorkspaceSelection {
  if (structure.runtime.source_state.status === 'unavailable') {
    return { ...structure.selection, selected_mod_path: null };
  }

  return {
    ...structure.selection,
    selected_mod_path: currentSelection.selectedModPath,
  };
}

function normalizedPathEquals(
  left: string | null | undefined,
  right: string | null | undefined,
): boolean {
  if (left == null || right == null) {
    return left == null && right == null;
  }

  return normalizeWorkspacePath(left).toLowerCase() === normalizeWorkspacePath(right).toLowerCase();
}

export function previewRequestIdentityMatches(
  request: WorkspacePreviewRequestInput,
  identity: WorkspacePreviewRequestIdentity,
): boolean {
  return (
    request.gameId === identity.game_id &&
    normalizedPathEquals(request.explorerSubPath, identity.explorer_sub_path) &&
    normalizedPathEquals(request.selectedModPath, identity.selected_mod_path)
  );
}

function buildPreviewSelectionForRuntimeEvent(
  currentSelection: WorkspaceViewModelSelectionInput,
  previewSelection: WorkspacePreviewSelection,
  currentPath: string[],
): WorkspaceSelection {
  return {
    selected_object_folder_path: currentSelection.selectedObjectFolderPath,
    explorer_sub_path: currentSelection.explorerSubPath ?? null,
    selected_mod_path: previewSelection.selected_mod_path,
    current_path: currentPath,
    reconciliation_status: previewSelection.reconciliation_status,
    reconciliation_reason: previewSelection.reconciliation_reason,
    affected_paths: previewSelection.affected_paths,
  };
}

export function shouldApplyWorkspacePreviewSelection(
  request: WorkspacePreviewRequestInput,
  currentSelection: WorkspaceViewModelSelectionInput,
  currentPath: string[],
  identity: WorkspacePreviewRequestIdentity,
  contextStatus: WorkspacePreviewContextStatus,
  previewSelection: WorkspacePreviewSelection,
  nowMs?: number,
): boolean {
  if (contextStatus !== 'ready' || !previewRequestIdentityMatches(request, identity)) {
    return false;
  }

  return shouldApplySelectionReconciledEvent(
    currentSelection,
    buildPreviewSelectionForRuntimeEvent(currentSelection, previewSelection, currentPath),
    nowMs,
  );
}

export function useWorkspaceStructure(options?: UseWorkspaceStructureOptions) {
  const currentSelection = useWorkspaceSelectionInput();
  const selection = useMemo<WorkspaceStructureSelectionInput>(
    () => ({
      selectedObjectFolderPath:
        options?.selectionOverrides?.selectedObjectFolderPath ??
        currentSelection.selectedObjectFolderPath,
      explorerSubPath:
        options?.selectionOverrides?.explorerSubPath ?? currentSelection.explorerSubPath,
    }),
    [
      currentSelection.explorerSubPath,
      currentSelection.selectedObjectFolderPath,
      options?.selectionOverrides?.explorerSubPath,
      options?.selectionOverrides?.selectedObjectFolderPath,
    ],
  );
  const filterInput = useWorkspaceFilterInput(options);
  const filter = buildWorkspaceViewModelFilter(filterInput);

  const query = useQuery<WorkspaceStructureViewModel>({
    queryKey: workspaceKeys.structure(
      filter,
      selection.selectedObjectFolderPath,
      selection.explorerSubPath,
    ),
    queryFn: () => commands.getWorkspaceStructure(buildWorkspaceStructureInput(filter, selection)),
    enabled: !!filterInput.gameId && (options?.enabled ?? true),
    staleTime: 30_000,
    refetchOnWindowFocus: false,
    placeholderData: keepPreviousData,
  });

  useEffect(() => {
    if (options?.selectionOverrides || !query.data || query.isPlaceholderData) {
      return;
    }

    const reconciledSelection = structureSelectionForRuntimeEvent(currentSelection, query.data);
    const nowMs = Date.now();
    if (!shouldApplySelectionReconciledEvent(currentSelection, reconciledSelection, nowMs)) {
      return;
    }

    if (
      !shouldRunSelectionReconciliationEffect({
        gameId: filterInput.gameId,
        selection: reconciledSelection,
      })
    ) {
      return;
    }

    dispatchWorkspaceRuntimeEvent(buildSelectionReconciledEvent(reconciledSelection));
  }, [
    currentSelection,
    filterInput.gameId,
    options?.selectionOverrides,
    query.data,
    query.isPlaceholderData,
  ]);

  return query;
}

export function useWorkspacePreview(options?: UseWorkspacePreviewOptions) {
  const { activeGame } = useActiveGame();
  const currentSelection = useWorkspaceSelectionInput();
  const currentPath = useWorkspaceRuntimeSelector((state) => state.currentPath);
  const queryClient = useQueryClient();
  const request = useMemo<WorkspacePreviewRequestInput>(
    () => ({
      gameId: activeGame?.id ?? null,
      explorerSubPath: currentSelection.explorerSubPath,
      selectedModPath: currentSelection.selectedModPath,
    }),
    [activeGame?.id, currentSelection.explorerSubPath, currentSelection.selectedModPath],
  );
  const enabled = Boolean(request.gameId && request.selectedModPath && (options?.enabled ?? true));

  const query = useQuery<WorkspacePreviewResult>({
    queryKey: workspaceKeys.preview(
      request.gameId ?? '',
      request.explorerSubPath,
      request.selectedModPath ?? '',
    ),
    queryFn: () => commands.getWorkspacePreview(buildWorkspacePreviewInput(request)),
    enabled,
    staleTime: 0,
    gcTime: 0,
    refetchOnWindowFocus: false,
  });

  useEffect(() => {
    if (!query.data || query.data.context_status !== 'context_stale') {
      return;
    }

    void queryClient.invalidateQueries({
      queryKey: workspaceKeys.structures,
      refetchType: 'active',
    });
  }, [query.data, queryClient]);

  useEffect(() => {
    if (!query.data) {
      return;
    }

    const nowMs = Date.now();
    if (
      !shouldApplyWorkspacePreviewSelection(
        request,
        currentSelection,
        currentPath,
        query.data.request_identity,
        query.data.context_status,
        query.data.selection,
        nowMs,
      )
    ) {
      return;
    }

    const reconciledSelection = buildPreviewSelectionForRuntimeEvent(
      currentSelection,
      query.data.selection,
      currentPath,
    );
    if (
      !shouldRunSelectionReconciliationEffect({
        gameId: request.gameId,
        selection: reconciledSelection,
      })
    ) {
      return;
    }

    dispatchWorkspaceRuntimeEvent(buildSelectionReconciledEvent(reconciledSelection));
  }, [currentPath, currentSelection, query.data, request]);

  return query;
}

/**
 * Transitional frontend alias for structure consumers. It no longer calls the
 * legacy workspace command and deliberately excludes selectedModPath from its key.
 */
export const useWorkspaceViewModel = useWorkspaceStructure;
