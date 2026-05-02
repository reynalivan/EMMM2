import type { WorkspaceObjectNode } from '../../types/workspace';
import { isWorkspaceSwitchChecked } from '../workspace-runtime/actions/workspaceSwitchPolicy';
import { buildWorkspaceObjectActionPolicy } from '../workspace-runtime/actions/workspaceActionPolicy';
import type { ContextMenuTarget } from './ObjectContextMenu';

export function buildObjectContextMenuTarget(obj: WorkspaceObjectNode): ContextMenuTarget {
  return {
    type: 'object',
    id: obj.id,
    name: obj.name,
    objectType: obj.object_type,
    isEnabled: isWorkspaceSwitchChecked(obj),
    enabledCount: obj.enabled_count,
    modCount: obj.mod_count,
    isPinned: obj.is_pinned,
    category: obj.object_type,
    capabilities: obj.capabilities,
    actionPolicy: buildWorkspaceObjectActionPolicy(obj),
  };
}
