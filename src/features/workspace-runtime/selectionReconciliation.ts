/**
 * Selection reconciliation guards for the workspace view model.
 *
 * The backend reports how it repaired a stale selection; the frontend has to
 * decide whether that report is news or just an echo of a rewrite it performed
 * itself moments ago. All of that is pure decision logic plus two short-lived
 * in-memory guards, kept out of the hook so it can be tested directly.
 */

import type { WorkspaceSelection } from '../../types/workspace';
import type { WorkspaceRuntimeEvent } from './state/workspaceEvents';
import {
  normalizeWorkspacePath,
  rewriteWorkspacePathValue,
  type WorkspacePathRewriteInput,
} from './pathRewrite';
import { pathBasename } from '../../lib/pathKey';

export interface WorkspaceViewModelSelectionInput {
  selectedObjectFolderPath: string | null;
  explorerSubPath: string | undefined;
  selectedModPath: string | null;
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

export function buildReconciliationMessage(reason: string | null): string {
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
  return pathBasename(normalizeWorkspacePath(path));
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
