import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';
import { commands } from '../../lib/bindings';
import { useActiveGame } from '../../hooks/useActiveGame';
import { useAppStore } from '../../stores/useAppStore';
import { toast } from '../../stores/useToastStore';
import { ItemStatus, type ObjectFilter } from '../../types/object';
import type { WorkspaceSelection, WorkspaceViewModel } from '../../types/workspace';
import {
  dispatchWorkspaceRuntimeEvent,
  useWorkspaceRuntimeSelector,
} from './state/workspaceStoreBridge';
import type { WorkspaceRuntimeEvent } from './state/workspaceEvents';
import {
  normalizeWorkspacePath,
  rewriteWorkspacePathValue,
  type WorkspacePathRewriteInput,
} from './pathRewrite';

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

interface RecentInternalRewrite extends WorkspacePathRewriteInput {
  recordedAtMs: number;
}

interface SelectionReconciliationEffectKeyInput {
  gameId: string | null;
  safeMode: boolean;
  selection: WorkspaceSelection;
}

const INTERNAL_REWRITE_TTL_MS = 5_000;
const RECONCILIATION_EFFECT_TTL_MS = 10_000;
const recentInternalRewrites: RecentInternalRewrite[] = [];
const seenSelectionReconciliationEffects = new Map<string, number>();

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

export function buildSelectionReconciledEvent(
  selection: WorkspaceSelection,
): Extract<WorkspaceRuntimeEvent, { type: 'SELECTION_RECONCILED' }> {
  return {
    type: 'SELECTION_RECONCILED',
    selectedObjectFolderPath: selection.selected_object_folder_path,
    explorerSubPath: selection.explorer_sub_path ?? undefined,
    selectedModPath: selection.selected_mod_path,
    currentPath: selection.current_path,
    reconciliationStatus: selection.reconciliation_status,
    reconciliationReason: selection.reconciliation_reason,
    affectedPaths: selection.affected_paths,
  };
}

function buildReconciliationMessage(reason: string | null): string {
  if (reason === 'source_unavailable') {
    return 'Workspace source is unavailable. Selection was cleared.';
  }

  return 'Workspace target changed on disk. Selection was updated.';
}

function normalizeSelectionPath(path: string | null | undefined): string | null {
  return path ? normalizeWorkspacePath(path) : null;
}

function pruneInternalRewrites(nowMs: number): void {
  const firstLiveIndex = recentInternalRewrites.findIndex(
    (rewrite) => nowMs - rewrite.recordedAtMs <= INTERNAL_REWRITE_TTL_MS,
  );
  if (firstLiveIndex <= 0) {
    if (firstLiveIndex === -1) {
      recentInternalRewrites.splice(0, recentInternalRewrites.length);
    }
    return;
  }

  recentInternalRewrites.splice(0, firstLiveIndex);
}

function pruneSelectionReconciliationEffects(nowMs: number): void {
  for (const [key, recordedAtMs] of seenSelectionReconciliationEffects.entries()) {
    if (nowMs - recordedAtMs > RECONCILIATION_EFFECT_TTL_MS) {
      seenSelectionReconciliationEffects.delete(key);
    }
  }
}

function normalizedLastSegment(path: string): string {
  const segments = normalizeWorkspacePath(path).split('/').filter(Boolean);
  return segments[segments.length - 1] ?? normalizeWorkspacePath(path);
}

function normalizeAffectedPaths(paths: string[]): string[] {
  return paths.map(normalizeWorkspacePath).sort();
}

function serializeSelection(selection: WorkspaceSelection): string {
  return JSON.stringify({
    selectedObjectFolderPath: normalizeSelectionPath(selection.selected_object_folder_path),
    explorerSubPath: normalizeSelectionPath(selection.explorer_sub_path),
    selectedModPath: normalizeSelectionPath(selection.selected_mod_path),
    currentPath: selection.current_path,
    reconciliationStatus: selection.reconciliation_status,
    reconciliationReason: selection.reconciliation_reason,
    affectedPaths: normalizeAffectedPaths(selection.affected_paths),
  });
}

function pathTouchesRewriteOldPath(path: string, rewrite: WorkspacePathRewriteInput): boolean {
  const normalizedPath = normalizeWorkspacePath(path);
  const rewritten = rewriteWorkspacePathValue(normalizedPath, [rewrite]);
  return !!rewritten && normalizeWorkspacePath(rewritten) !== normalizedPath;
}

function pathTouchesRewriteNewPath(path: string, rewrite: WorkspacePathRewriteInput): boolean {
  const normalizedPath = normalizeWorkspacePath(path);
  const normalizedNewPath = normalizeWorkspacePath(rewrite.newPath);
  const newName = normalizedLastSegment(rewrite.newPath);
  const segments = normalizedPath.split('/').filter(Boolean);

  return (
    normalizedPath === normalizedNewPath ||
    normalizedPath.startsWith(`${normalizedNewPath}/`) ||
    segments.includes(newName)
  );
}

function reconciliationCoveredByRecentInternalRewrite(
  selection: WorkspaceViewModelSelectionInput,
  reconciledSelection: WorkspaceSelection,
  nowMs: number,
): boolean {
  if (reconciledSelection.reconciliation_status === 'unchanged') {
    return false;
  }

  pruneInternalRewrites(nowMs);
  if (recentInternalRewrites.length === 0 || reconciledSelection.affected_paths.length === 0) {
    return false;
  }

  const currentPaths = [
    selection.selectedModPath,
    selection.explorerSubPath,
    selection.selectedObjectFolderPath,
  ].filter((path): path is string => !!path);

  if (currentPaths.length === 0) {
    return false;
  }

  return recentInternalRewrites.some((rewrite) => {
    const affectedOldPath = reconciledSelection.affected_paths.some((path) =>
      pathTouchesRewriteOldPath(path, rewrite),
    );
    if (!affectedOldPath) {
      return false;
    }

    return currentPaths.some((path) => pathTouchesRewriteNewPath(path, rewrite));
  });
}

export function recordInternalWorkspacePathRewrites(
  rewrites: WorkspacePathRewriteInput[],
  nowMs: number,
): void {
  pruneInternalRewrites(nowMs);
  for (const rewrite of rewrites) {
    recentInternalRewrites.push({
      oldPath: normalizeWorkspacePath(rewrite.oldPath),
      newPath: normalizeWorkspacePath(rewrite.newPath),
      recordedAtMs: nowMs,
    });
  }
}

export function shouldRunSelectionReconciliationEffect(
  input: SelectionReconciliationEffectKeyInput,
): boolean {
  const nowMs = Date.now();
  pruneSelectionReconciliationEffects(nowMs);
  const key = JSON.stringify({
    gameId: input.gameId,
    safeMode: input.safeMode,
    selection: serializeSelection(input.selection),
  });
  if (seenSelectionReconciliationEffects.has(key)) {
    return false;
  }

  seenSelectionReconciliationEffects.set(key, nowMs);
  return true;
}

export function shouldShowSelectionReconciliationToast(
  selection: WorkspaceViewModelSelectionInput,
  reconciledSelection: WorkspaceSelection,
  nowMs: number,
): boolean {
  if (reconciledSelection.reconciliation_status === 'unchanged') {
    return false;
  }

  return !reconciliationCoveredByRecentInternalRewrite(selection, reconciledSelection, nowMs);
}

export function resetWorkspaceSelectionReconciliationGuardsForTest(): void {
  recentInternalRewrites.splice(0, recentInternalRewrites.length);
  seenSelectionReconciliationEffects.clear();
}

export function shouldApplySelectionReconciledEvent(
  selection: WorkspaceViewModelSelectionInput,
  reconciledSelection: WorkspaceSelection,
  nowMs?: number,
): boolean {
  const currentTimeMs = nowMs ?? Date.now();
  const nextExplorerSubPath = reconciledSelection.explorer_sub_path ?? undefined;
  const reconciliationChanged = reconciledSelection.reconciliation_status !== 'unchanged';
  const selectionMatches =
    selection.selectedObjectFolderPath === reconciledSelection.selected_object_folder_path &&
    selection.explorerSubPath === nextExplorerSubPath &&
    normalizeSelectionPath(selection.selectedModPath) ===
      normalizeSelectionPath(reconciledSelection.selected_mod_path);

  if (selectionMatches && !reconciliationChanged) {
    return false;
  }

  if (
    reconciliationChanged &&
    reconciliationCoveredByRecentInternalRewrite(selection, reconciledSelection, currentTimeMs)
  ) {
    return false;
  }

  if (
    !reconciliationChanged &&
    selection.selectedModPath &&
    normalizeSelectionPath(selection.selectedModPath) !==
      normalizeSelectionPath(reconciledSelection.selected_mod_path)
  ) {
    return false;
  }

  return true;
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
    const nowMs = Date.now();
    if (!shouldApplySelectionReconciledEvent(selection, reconciledSelection, nowMs)) {
      return;
    }

    if (
      !shouldRunSelectionReconciliationEffect({
        gameId: filterInput.gameId,
        safeMode: filterInput.safeMode,
        selection: reconciledSelection,
      })
    ) {
      return;
    }

    dispatchWorkspaceRuntimeEvent(buildSelectionReconciledEvent(reconciledSelection));
    if (shouldShowSelectionReconciliationToast(selection, reconciledSelection, nowMs)) {
      toast.info(buildReconciliationMessage(reconciledSelection.reconciliation_reason), 4000);
    }
  }, [filterInput.gameId, filterInput.safeMode, query.data, query.isPlaceholderData, selection]);

  return query;
}
