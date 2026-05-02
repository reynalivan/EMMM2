import type { ObjectFilter, ObjectSummary } from './object';
import type { ConflictGroup, ModFolder } from './mod';

export interface WorkspaceViewModelInput {
  filter: ObjectFilter;
  selected_object_folder_path?: string | null;
  explorer_sub_path?: string | null;
  selected_mod_path?: string | null;
}

export interface WorkspaceSelection {
  selected_object_folder_path: string | null;
  explorer_sub_path: string | null;
  selected_mod_path: string | null;
  current_path: string[];
}

export type WorkspaceNodeKind = 'object' | 'container' | 'terminal_mod' | 'inactive_branch';

export type WorkspaceDisplayMode =
  | 'container_folder'
  | 'mod_pack'
  | 'variant'
  | 'flat_mod'
  | 'internal_assets'
  | 'unknown';

export type WorkspaceTypeChip = 'mod_pack' | 'variant' | 'flat_mod';

export type WorkspaceWarningState = 'none' | 'warning';
export type WorkspaceSwitchState =
  | 'enabled'
  | 'disabled'
  | 'effectively_disabled'
  | 'blocked_by_ancestor';
export type WorkspaceSwitchPolicyKey = 'mod' | 'object' | 'blocked';
export type WorkspaceReasonCode = 'disabled_by_container' | 'object_folder_disabled';
export type WorkspaceWarningCode = 'folder_warning' | 'inactive_reason' | 'naming_conflict';

export interface WorkspaceCapabilities {
  can_toggle: boolean;
  can_rename: boolean;
  can_delete: boolean;
  can_move: boolean;
  can_toggle_safe: boolean;
  can_sync: boolean;
  can_enable_only_this: boolean;
  can_pin: boolean;
  can_edit_metadata: boolean;
  can_reveal_in_explorer: boolean;
  can_move_category: boolean;
  can_open_in_explorer: boolean;
}

export interface WorkspaceReason {
  code: WorkspaceReasonCode;
  args: Record<string, string>;
}

export interface WorkspaceWarning {
  code: WorkspaceWarningCode;
  args: Record<string, string>;
  state: WorkspaceWarningState;
}

export interface WorkspaceNodeBase {
  node_kind: WorkspaceNodeKind;
  display_mode: WorkspaceDisplayMode;
  type_chip: WorkspaceTypeChip | null;
  display_name: string;
  is_effectively_active: boolean;
  inactive_reason: WorkspaceReason | null;
  warning_state: WorkspaceWarningState;
  primary_warning: WorkspaceWarning | null;
  switch_state: WorkspaceSwitchState;
  switch_reason: WorkspaceReason | null;
  switch_policy_key: WorkspaceSwitchPolicyKey;
  capabilities: WorkspaceCapabilities;
}

export interface WorkspaceExplorerNode extends ModFolder, WorkspaceNodeBase {
  ancestor_disabled: boolean;
  can_navigate: boolean;
}

export interface WorkspaceObjectNode extends ObjectSummary, WorkspaceNodeBase {
  node_kind: 'object';
}

export type WorkspaceNode = WorkspaceExplorerNode | WorkspaceObjectNode;

export function isWorkspaceObjectNode(
  node: WorkspaceNode | null | undefined,
): node is WorkspaceObjectNode {
  if (!node) {
    return false;
  }

  return node.node_kind === 'object';
}

export function isWorkspaceExplorerNode(
  node: WorkspaceNode | null | undefined,
): node is WorkspaceExplorerNode {
  if (!node) {
    return false;
  }

  return node.node_kind !== 'object';
}

export interface WorkspaceExplorer {
  self_node_type: string | null;
  self_node_kind: WorkspaceNodeKind;
  self_display_mode: WorkspaceDisplayMode;
  self_type_chip: WorkspaceTypeChip | null;
  self_is_mod: boolean;
  self_is_enabled: boolean;
  self_is_effectively_active: boolean;
  self_owner_object_id: string | null;
  self_owner_object_folder_path: string | null;
  self_classification_reasons: string[];
  children: WorkspaceExplorerNode[];
  conflicts: ConflictGroup[];
  ancestor_disabled_by: string | null;
  ancestor_disabled_path: string | null;
  inactive_reason: WorkspaceReason | null;
}

export interface WorkspaceModInfoSummary {
  actual_name: string;
  author: string;
  version: string;
  description: string;
  is_safe: boolean;
  is_favorite: boolean;
  has_info_json: boolean;
}

export interface WorkspaceIniSummary {
  file_count: number;
  file_names: string[];
}

export interface WorkspaceImageSummary {
  image_count: number;
  primary_image_path: string | null;
}

export interface WorkspaceWarningSummary {
  state: WorkspaceWarningState;
  messages: WorkspaceWarning[];
}

export interface WorkspacePreview {
  selected_path: string | null;
  selected_node: WorkspaceNode | null;
  is_flat_mod_root: boolean;
  display_title: string | null;
  display_subtitle: string | null;
  mod_info_summary: WorkspaceModInfoSummary | null;
  ini_summary: WorkspaceIniSummary | null;
  image_summary: WorkspaceImageSummary | null;
  warning_summary: WorkspaceWarningSummary;
}

export interface WorkspaceRuntime {
  game_id: string;
  safe_mode: boolean;
}

export type WorkspaceSwitchTargetKind = 'mod_path' | 'object_id';
export type WorkspaceSwitchResolution =
  | 'normal'
  | 'force_enable'
  | 'enable_only_this'
  | 'enable_parent_then_continue';
export type WorkspaceSwitchOriginSurface =
  | 'folder_grid'
  | 'preview'
  | 'object_list'
  | 'collections'
  | 'corridor';
export type WorkspaceSwitchStatus = 'applied' | 'requires_duplicate_resolution' | 'noop';
export type WorkspaceRefreshScope =
  | 'workspaceChanged'
  | 'objectRowsChanged'
  | 'folderStructureChanged'
  | 'folderMetadataChanged'
  | 'previewChanged'
  | 'thumbnailChanged'
  | 'conflictsChanged'
  | 'corridorChanged'
  | 'collectionsChanged'
  | 'dashboardChanged'
  | 'activeKeybindingsChanged'
  | 'trashChanged'
  | 'settingsChanged'
  | 'browserDownloadsChanged'
  | 'browserImportQueueChanged'
  | 'browserHomepageChanged'
  | 'dedupChanged'
  | 'dedupReportChanged'
  | 'scannerChanged'
  | 'pinsChanged';

export interface WorkspaceSwitchTarget {
  kind: WorkspaceSwitchTargetKind;
  value: string;
}

export interface WorkspaceSwitchDuplicate {
  mod_id: string;
  object_id: string;
  folder_path: string;
  actual_name: string;
  is_variant: boolean;
  parent_path: string;
}

export interface WorkspacePathRewrite {
  old_path: string;
  new_path: string;
}

export interface WorkspaceImpact {
  rewrites: WorkspacePathRewrite[];
  cleared_targets: string[];
  changed_object_ids: string[];
  changed_folder_paths: string[];
  refresh_scopes: WorkspaceRefreshScope[];
  projection_dirty: boolean;
  warnings: string[];
}

export interface WorkspaceSwitchInput {
  game_id: string;
  target: WorkspaceSwitchTarget;
  desired_enabled: boolean;
  resolution: WorkspaceSwitchResolution;
  origin_surface: WorkspaceSwitchOriginSurface;
}

export interface WorkspaceSwitchResult {
  status: WorkspaceSwitchStatus;
  primary_path: string | null;
  changed_folder_paths: string[];
  changed_object_ids: string[];
  duplicates: WorkspaceSwitchDuplicate[];
  impact: WorkspaceImpact;
}

export interface WorkspaceViewModel {
  objects: WorkspaceObjectNode[];
  explorer: WorkspaceExplorer;
  preview: WorkspacePreview;
  selection: WorkspaceSelection;
  runtime: WorkspaceRuntime;
}
