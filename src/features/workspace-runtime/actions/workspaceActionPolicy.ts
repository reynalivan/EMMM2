import type { WorkspaceObjectNode } from '../../../types/workspace';

export interface WorkspaceObjectActionPolicy {
  canEdit: boolean;
  canReveal: boolean;
  canPin: boolean;
  canMoveCategory: boolean;
  canSync: boolean;
  canDelete: boolean;
  canEnable: boolean;
  canDisable: boolean;
}

export function buildWorkspaceObjectActionPolicy(
  node: Pick<WorkspaceObjectNode, 'capabilities' | 'switch_state'>,
): WorkspaceObjectActionPolicy {
  const desiredEnabled = node.switch_state !== 'enabled';

  return {
    canEdit: node.capabilities.can_edit_metadata || node.capabilities.can_rename,
    canReveal: node.capabilities.can_reveal_in_explorer,
    canPin: node.capabilities.can_pin,
    canMoveCategory: node.capabilities.can_move_category,
    canSync: node.capabilities.can_sync,
    canDelete: node.capabilities.can_delete,
    canEnable: node.capabilities.can_toggle && desiredEnabled,
    canDisable: node.capabilities.can_toggle && !desiredEnabled,
  };
}
