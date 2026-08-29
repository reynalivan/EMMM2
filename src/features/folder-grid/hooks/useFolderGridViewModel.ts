import { useMemo } from 'react';
import { useActiveConflicts } from '../../../hooks/useFolderMutations';
import { useAppStore } from '../../../stores/useAppStore';
import type { WorkspaceExplorerNode } from '../../../types/workspace';
import { normalizeWorkspacePath } from '../../workspace-runtime/pathRewrite';

interface UseFolderGridViewModelInput {
  sortedFolders: WorkspaceExplorerNode[];
  sourceUnavailableMessage: string | null;
  recoveryStatus: 'ready' | 'syncing' | 'failed';
}

export function useFolderGridViewModel({
  sortedFolders,
  sourceUnavailableMessage,
  recoveryStatus,
}: UseFolderGridViewModelInput) {
  const { data: conflicts = [] } = useActiveConflicts();
  const activePane = useAppStore((state) => state.activePane);
  const activeGameId = useAppStore((state) => state.activeGameId);
  const diskSourceUnavailableMessage = useAppStore((state) =>
    activeGameId ? (state.diskReconcileByGame[activeGameId]?.unavailable ?? null) : null,
  );
  const setActivePane = useAppStore((state) => state.setActivePane);
  const isIgnoreManagementOpen = useAppStore((state) => state.isIgnoreManagementOpen);
  const setIsIgnoreManagementOpen = useAppStore((state) => state.setIgnoreManagementOpen);
  const folderNameConflicts = useAppStore((state) =>
    activeGameId ? (state.folderConflictsByGame[activeGameId] ?? []) : [],
  );
  const hasBlockingDiskReport = useAppStore((state) =>
    activeGameId ? (state.renameConfirmationsByGame[activeGameId]?.length ?? 0) > 0 : false,
  );

  const conflictPathSet = useMemo(() => {
    const paths = new Set<string>();
    for (const conflict of conflicts) {
      if (conflict.mod_paths.length <= 1) {
        continue;
      }
      for (const path of conflict.mod_paths) {
        paths.add(normalizeWorkspacePath(path));
      }
    }
    for (const group of folderNameConflicts) {
      for (const candidate of group.candidates) {
        paths.add(normalizeWorkspacePath(candidate.path));
      }
    }
    return paths;
  }, [conflicts, folderNameConflicts]);
  const folderConflictScopes = useMemo(
    () =>
      folderNameConflicts.flatMap((group) => group.candidates.map((candidate) => candidate.path)),
    [folderNameConflicts],
  );

  const workspaceSourceUnavailableMessage =
    sourceUnavailableMessage ?? diskSourceUnavailableMessage;
  const mutationsDisabled =
    recoveryStatus === 'syncing' ||
    Boolean(workspaceSourceUnavailableMessage) ||
    hasBlockingDiskReport;

  const handleSelectAll = () => {
    useAppStore.getState().setGridSelection(new Set(sortedFolders.map((folder) => folder.path)));
  };

  return {
    visibleFolders: sortedFolders,
    conflictPathSet,
    folderConflictScopes,
    activePane,
    setActivePane,
    isIgnoreManagementOpen,
    setIsIgnoreManagementOpen,
    workspaceSourceUnavailableMessage,
    recoveryStatus,
    mutationsDisabled,
    handleSelectAll,
  };
}
