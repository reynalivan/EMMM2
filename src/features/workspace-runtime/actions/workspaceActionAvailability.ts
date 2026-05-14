import type { WorkspaceCapabilities, WorkspaceNode } from '../../../types/workspace';

export const DEFAULT_SOURCE_UNAVAILABLE_MESSAGE = 'Workspace source is unavailable.';

const DISABLED_CAPABILITIES: WorkspaceCapabilities = {
  can_toggle: false,
  can_rename: false,
  can_delete: false,
  can_move: false,
  can_toggle_safe: false,
  can_sync: false,
  can_enable_only_this: false,
  can_pin: false,
  can_edit_metadata: false,
  can_reveal_in_explorer: false,
  can_move_category: false,
  can_open_in_explorer: false,
};

export function areWorkspaceMutationsDisabled(sourceUnavailableMessage: string | null): boolean {
  return Boolean(sourceUnavailableMessage);
}

export function maskWorkspaceCapabilities(
  capabilities: WorkspaceCapabilities,
  mutationsDisabled: boolean,
): WorkspaceCapabilities {
  if (!mutationsDisabled) {
    return capabilities;
  }

  return DISABLED_CAPABILITIES;
}

export function maskWorkspaceNodeCapabilities<TNode extends WorkspaceNode>(
  node: TNode,
  mutationsDisabled: boolean,
): TNode {
  if (!mutationsDisabled) {
    return node;
  }

  return {
    ...node,
    capabilities: DISABLED_CAPABILITIES,
  };
}
