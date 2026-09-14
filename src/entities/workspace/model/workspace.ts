export type {
  WorkspaceCapabilities,
  WorkspaceDisplayMode,
  WorkspaceExplorer,
  WorkspaceExplorerNode,
  WorkspaceImageSummary,
  WorkspaceImpact,
  WorkspaceIniSummary,
  WorkspaceModInfoSummary,
  WorkspaceNavigationSelection,
  WorkspaceNode,
  WorkspaceNodeKind,
  WorkspaceObjectNode,
  WorkspaceParentEnableImpact,
  WorkspaceParentEnableParent,
  WorkspaceParentEnableRequirement,
  WorkspacePathRewrite,
  WorkspacePreview,
  WorkspacePreviewContextStatus,
  WorkspacePreviewInput,
  WorkspacePreviewRequestIdentity,
  WorkspacePreviewResult,
  WorkspacePreviewSelection,
  WorkspaceReason,
  WorkspaceReasonCode,
  WorkspaceRecoveryStatus,
  WorkspaceRefreshScope,
  WorkspaceRuntime,
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
  WorkspaceStructureInput,
  WorkspaceStructureViewModel,
  WorkspaceWarning,
  WorkspaceWarningCode,
  WorkspaceWarningState,
} from '@/shared/api/tauri/bindings.gen';

import type {
  WorkspaceExplorerNode,
  WorkspaceNavigationSelection,
  WorkspaceNode,
} from '@/shared/api/tauri/bindings.gen';

/**
 * The frontend combines the navigation selection from the structure read model
 * with preview selection before dispatching a single runtime store event.
 */
export type WorkspaceSelection = WorkspaceNavigationSelection & {
  selected_mod_path: string | null;
};

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
