import type { DiskReconcileResult } from '../../lib/bindings';
import { useAppStore } from '../../stores/useAppStore';
import type { GameConfig } from '../../types/game';
import { dispatchWorkspaceRuntimeEvent } from '../workspace-runtime/state/workspaceStoreBridge';
import { isSameOrDescendantPath, joinModPath, normalizePath, rewritePath } from './pathUtils';

export function buildDiskReconcilePathRewrites(
  result: DiskReconcileResult,
  activeGame: GameConfig | null,
): Array<{ oldPath: string; newPath: string }> {
  const modsPath = activeGame?.mod_path;
  const rewrites: Array<{ oldPath: string; newPath: string }> = [];

  for (const update of result.path_updates) {
    if (update.kind !== 'Mod' || !modsPath) {
      rewrites.push({
        oldPath: update.from,
        newPath: update.to,
      });
      continue;
    }

    const absoluteFrom = joinModPath(modsPath, update.from);
    const absoluteTo = joinModPath(modsPath, update.to);
    rewrites.push({
      oldPath: absoluteFrom,
      newPath: absoluteTo,
    });
  }

  return rewrites;
}

export function clearStaleSelections(
  result: DiskReconcileResult,
  activeGame: GameConfig | null,
): void {
  const appStore = useAppStore.getState();
  const modsPath = activeGame?.mod_path;

  if (result.cleared_selection_paths.length === 0) {
    return;
  }

  const invalidatedPaths = [...result.cleared_selection_paths];
  if (modsPath) {
    invalidatedPaths.push(
      ...result.cleared_selection_paths.map((path) => joinModPath(modsPath, path)),
    );
  }

  dispatchWorkspaceRuntimeEvent({
    type: 'TARGETS_INVALIDATED',
    paths: invalidatedPaths,
    resetExplorer: true,
  });

  if (!modsPath || appStore.gridSelection.size === 0) {
    return;
  }

  const selectedPaths = Array.from(appStore.gridSelection);
  const shouldClearGridSelection = result.cleared_selection_paths.some((path) => {
    const absoluteRoot = joinModPath(modsPath, path);
    return selectedPaths.some((selectedPath) => isSameOrDescendantPath(selectedPath, absoluteRoot));
  });

  if (shouldClearGridSelection) {
    appStore.clearGridSelection();
  }
}

export function isPreviewAffected(
  result: DiskReconcileResult,
  activeGame: GameConfig | null,
): boolean {
  if (!activeGame?.mod_path) {
    return false;
  }

  const appStore = useAppStore.getState();
  const selectedPaths = Array.from(appStore.gridSelection);
  const selectedModPath =
    selectedPaths.length > 0 ? selectedPaths[selectedPaths.length - 1] : undefined;
  if (!selectedModPath) {
    return false;
  }

  const selectedPath = normalizePath(selectedModPath);
  const selectedObjectPath = appStore.selectedObjectFolderPath
    ? normalizePath(appStore.selectedObjectFolderPath)
    : null;

  for (const update of result.path_updates) {
    if (update.kind === 'Mod') {
      const absoluteFrom = joinModPath(activeGame.mod_path, update.from);
      const absoluteTo = joinModPath(activeGame.mod_path, update.to);
      if (
        rewritePath(selectedPath, absoluteFrom, absoluteTo) ||
        isSameOrDescendantPath(selectedPath, absoluteTo)
      ) {
        return true;
      }
      continue;
    }

    const objectRewrite = selectedObjectPath
      ? rewritePath(selectedObjectPath, update.from, update.to)
      : null;
    if (
      objectRewrite ||
      (selectedObjectPath && isSameOrDescendantPath(selectedObjectPath, update.to))
    ) {
      return true;
    }
  }

  for (const clearedPath of result.cleared_selection_paths) {
    const absoluteRoot = joinModPath(activeGame.mod_path, clearedPath);
    if (isSameOrDescendantPath(selectedPath, absoluteRoot)) {
      return true;
    }
  }

  for (const changedRoot of result.changed_roots) {
    const absoluteRoot = joinModPath(activeGame.mod_path, changedRoot);
    if (isSameOrDescendantPath(selectedPath, absoluteRoot)) {
      return true;
    }
  }

  for (const thumbnailRoot of result.thumbnail_roots) {
    const absoluteRoot = joinModPath(activeGame.mod_path, thumbnailRoot);
    if (isSameOrDescendantPath(selectedPath, absoluteRoot)) {
      return true;
    }
  }

  return false;
}
