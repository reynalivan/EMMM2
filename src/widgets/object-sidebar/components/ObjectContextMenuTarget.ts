import type { WorkspaceObjectNode } from '@/entities/workspace/model/workspace';
import { isWorkspaceSwitchChecked } from '@/features/workspace-runtime/actions/workspaceSwitchPolicy';
import { buildWorkspaceObjectActionPolicy } from '@/features/workspace-runtime/actions/workspaceActionPolicy';
import type { ContextMenuTarget } from './ObjectContextMenu';

export function buildObjectContextMenuTarget(obj: WorkspaceObjectNode): ContextMenuTarget {
  return {
    id: obj.id,
    name: obj.name,
    isEnabled: isWorkspaceSwitchChecked(obj),
    isPinned: obj.is_pinned,
    capabilities: obj.capabilities,
    actionPolicy: buildWorkspaceObjectActionPolicy(obj),
  };
}
