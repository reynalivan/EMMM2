import { useMemo } from 'react';
import { useActiveConflicts } from './useFolderMutations';
import { useAppStore } from '@/app/store';
import type { WorkspaceExplorerNode } from '@/entities/workspace';
import type { FolderNameConflictGroup } from '@/shared/api/tauri/bindings';
import { normalizeWorkspacePath } from '@/features/workspace-runtime';

const EMPTY_FOLDER_CONFLICTS: FolderNameConflictGroup[] = [];

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
  const activation = useAppStore((state) =>
    activeGameId ? state.gameActivationByGame?.[activeGameId] : undefined,
  );
  const setActivePane = useAppStore((state) => state.setActivePane);
  const isIgnoreManagementOpen = useAppStore((state) => state.isIgnoreManagementOpen);
  const setIsIgnoreManagementOpen = useAppStore((state) => state.setIgnoreManagementOpen);
  const folderNameConflicts = useAppStore((state) =>
    activeGameId
      ? (state.folderConflictsByGame[activeGameId] ?? EMPTY_FOLDER_CONFLICTS)
      : EMPTY_FOLDER_CONFLICTS,
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

  const activationFailed =
    activation?.phase === 'failed' || activation?.phase === 'source_unavailable';
  const effectiveRecoveryStatus =
    activation?.phase === 'syncing' ? 'syncing' : activationFailed ? 'failed' : recoveryStatus;
  const workspaceSourceUnavailableMessage =
    sourceUnavailableMessage ?? diskSourceUnavailableMessage ?? activation?.error ?? null;
  const mutationsDisabled =
    effectiveRecoveryStatus !== 'ready' ||
    Boolean(workspaceSourceUnavailableMessage) ||
    hasBlockingDiskReport;

  return {
    visibleFolders: sortedFolders,
    conflictPathSet,
    folderConflictScopes,
    activePane,
    setActivePane,
    isIgnoreManagementOpen,
    setIsIgnoreManagementOpen,
    workspaceSourceUnavailableMessage,
    recoveryStatus: effectiveRecoveryStatus,
    mutationsDisabled,
  };
}
