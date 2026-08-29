import type { SafetyFilter } from '../../../stores/appStore/explorerSlice';
import type { WorkspaceExplorerNode } from '../../../types/workspace';

export function filterFoldersBySafety(
  folders: WorkspaceExplorerNode[],
  filter: SafetyFilter,
): WorkspaceExplorerNode[] {
  if (filter === 'all') {
    return folders;
  }

  return folders.filter((folder) =>
    filter === 'safe' ? folder.contains_safe_mods : folder.contains_unsafe_mods,
  );
}
