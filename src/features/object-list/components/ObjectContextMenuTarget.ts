import type { WorkspaceObjectNode } from '../../../types/workspace';
import { isWorkspaceSwitchChecked } from '../../workspace-runtime/actions/workspaceSwitchPolicy';
import { buildWorkspaceObjectActionPolicy } from '../../workspace-runtime/actions/workspaceActionPolicy';
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
