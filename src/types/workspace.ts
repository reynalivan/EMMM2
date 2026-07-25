export type {
  WorkspaceCapabilities,
  WorkspaceDisplayMode,
  WorkspaceExplorer,
  WorkspaceExplorerNode,
  WorkspaceImageSummary,
  WorkspaceImpact,
  WorkspaceIniSummary,
  WorkspaceModInfoSummary,
  WorkspaceNode,
  WorkspaceNodeKind,
  WorkspaceObjectNode,
  WorkspacePathRewrite,
  WorkspacePreview,
  WorkspaceReason,
  WorkspaceReasonCode,
  WorkspaceRefreshScope,
  WorkspaceRuntime,
  WorkspaceSelection,
  WorkspaceSelectionReconciliationReason,
  WorkspaceSelectionReconciliationStatus,
  WorkspaceSourceState,
  WorkspaceSourceStatus,
  WorkspaceSwitchDuplicate,
  WorkspaceSwitchInput,
  WorkspaceSwitchOriginSurface,
  WorkspaceSwitchPolicyKey,
  WorkspaceSwitchResolution,
  WorkspaceSwitchResult,
  WorkspaceSwitchState,
  WorkspaceSwitchStatus,
  WorkspaceSwitchTarget,
  WorkspaceSwitchTargetKind,
  WorkspaceTypeChip,
  WorkspaceViewModel,
  WorkspaceViewModelInput,
  WorkspaceWarning,
  WorkspaceWarningCode,
  WorkspaceWarningState,
} from '../lib/bindings.gen';

import type { WorkspaceExplorerNode, WorkspaceNode } from '../lib/bindings.gen';

/**
 * Frontend-only shared shape of workspace nodes (Rust flattens these fields
 * into each concrete node type, so there is no generated counterpart).
 */
export type WorkspaceNodeBase = Pick<
  WorkspaceExplorerNode,
  | 'node_kind'
  | 'display_mode'
  | 'type_chip'
  | 'display_name'
  | 'is_effectively_active'
  | 'inactive_reason'
  | 'warning_state'
  | 'primary_warning'
  | 'switch_state'
  | 'switch_reason'
  | 'switch_policy_key'
  | 'capabilities'
>;

export function isWorkspaceExplorerNode(
  node: WorkspaceNode | null | undefined,
): node is WorkspaceExplorerNode {
  if (!node) {
    return false;
  }

  return node.node_kind !== 'object';
}
