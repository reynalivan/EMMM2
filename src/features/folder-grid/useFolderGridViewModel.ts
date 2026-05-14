import { useMemo } from 'react';
import { useActiveConflicts } from '../../hooks/useFolderMutations';
import { useAppStore } from '../../stores/useAppStore';
import type { WorkspaceExplorerNode } from '../../types/workspace';
import { areWorkspaceMutationsDisabled } from '../workspace-runtime/actions/workspaceActionAvailability';

interface UseFolderGridViewModelInput {
  sortedFolders: WorkspaceExplorerNode[];
  sourceUnavailableMessage: string | null;
}

export function useFolderGridViewModel({
  sortedFolders,
  sourceUnavailableMessage,
}: UseFolderGridViewModelInput) {
  const { data: conflicts = [] } = useActiveConflicts();
  const activePane = useAppStore((state) => state.activePane);
  const activeGameId = useAppStore((state) => state.activeGameId);
  const diskSourceUnavailableMessage = useAppStore((state) =>
    activeGameId ? (state.diskSourceUnavailableByGame[activeGameId] ?? null) : null,
  );
  const setActivePane = useAppStore((state) => state.setActivePane);
  const isIgnoreManagementOpen = useAppStore((state) => state.isIgnoreManagementOpen);
  const setIsIgnoreManagementOpen = useAppStore((state) => state.setIgnoreManagementOpen);

  const conflictPathSet = useMemo(() => {
    const paths = new Set<string>();
    for (const conflict of conflicts) {
      if (conflict.mod_paths.length <= 1) {
        continue;
      }
      for (const path of conflict.mod_paths) {
        paths.add(path.replace(/\\/g, '/'));
      }
    }
    return paths;
  }, [conflicts]);

  const workspaceSourceUnavailableMessage =
    sourceUnavailableMessage ?? diskSourceUnavailableMessage;
  const mutationsDisabled = areWorkspaceMutationsDisabled(workspaceSourceUnavailableMessage);

  const handleSelectAll = () => {
    useAppStore.getState().setGridSelection(new Set(sortedFolders.map((folder) => folder.path)));
  };

  return {
    visibleFolders: sortedFolders,
    conflictPathSet,
    activePane,
    setActivePane,
    isIgnoreManagementOpen,
    setIsIgnoreManagementOpen,
    workspaceSourceUnavailableMessage,
    mutationsDisabled,
    handleSelectAll,
  };
}
