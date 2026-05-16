// ---------------------------------------------------------------------------
// v2 Domain Types — Direct mirrors of Rust `domain/` structs
// ---------------------------------------------------------------------------

import type { WorkspacePathRewrite } from './workspace';

export type CollectionKind = 'named' | 'undo_snapshot' | 'unsaved';
export type MemberKind = 'mod' | 'nested' | 'object' | 'root';

export type SwitchResult = {
  success: boolean;
  active_safe: boolean;
  mods_disabled: number;
  mods_restored: number;
  new_signature: string;
  warnings: string[];
  restored_collection_id?: string | null;
};

export type CorridorSnapshot = {
  game_id: string;
  is_safe: boolean;
  signature?: string;
  mod_count?: number;
  timestamp?: string;
  active_collection_id: string | null;
  active_collection_name: string | null;
  active_collection_is_unsaved: boolean;
  is_dirty: boolean;
  undo_collection_id: string | null;
  current_signature: string;
  last_switched_at?: string | null;
  created_at?: string | null;
  current_mods: CollectionMod[];
  current_objects: CollectionObject[];
  current_tree_nodes: PreviewTreeNode[];
  projected_state: ProjectedCollectionState;
};

export type CorridorSwitchPreview = {
  leaving_state_name: string | null;
  leaving_state_is_unsaved: boolean;
  leaving_state_is_safe: boolean;
  leaving_mods: CollectionMod[];
  leaving_objects: CollectionObject[];
  leaving_tree_nodes: PreviewTreeNode[];
  leaving_projected_state: ProjectedCollectionState;
  target_state_name: string | null;
  target_state_is_unsaved: boolean;
  target_state_is_safe: boolean;
  target_state_kind: string;
  target_mods: CollectionMod[];
  target_objects: CollectionObject[];
  target_tree_nodes: PreviewTreeNode[];
  target_projected_state: ProjectedCollectionState;
};

export type CollectionSummary = {
  id: string;
  name: string;
  is_safe: boolean;
  is_unsaved: boolean;
  is_active: boolean;
  is_undo_target: boolean;
  signature: string | null;
  updated_at: string;
  member_count: number;
  mod_count: number;
  game_id?: string;
  created_at?: number;
};

export type CollectionMod = (
  | {
      kind: 'mod';
    }
  | {
      kind: 'nested';
    }
) & {
  collection_id: string;
  mod_id: string | null;
  mod_path: string;
  path_key: string | null;
  mod_path_key: string | null;
  object_id: string;
  display_name: string;
  preview_path: string | null;
  node_type: string | null;
  warnings: string[];
  is_enabled: boolean;
};

export type CollectionObject = {
  kind: 'object';
  collection_id: string;
  object_id: string;
  is_enabled: boolean;
  display_name: string;
  path_key: string;
};

export type CollectionRoot = {
  kind: 'root';
  collection_id: string;
  root_path: string;
  root_path_key: string;
  display_name: string;
  display_name_key: string;
  object_id: string | null;
  object_name: string | null;
  object_type: string | null;
  root_kind: string;
  is_safe: boolean;
  is_enabled: boolean;
  thumbnail_hint: string | null;
  corridor_source: string | null;
  path_key: string;
};

export type CollectionMember = CollectionMod | CollectionObject | CollectionRoot;

export type PreviewTreeNodeKind = 'object' | 'folder' | 'mod';

export type PreviewTreeNode = {
  kind: PreviewTreeNodeKind;
  id: string;
  name: string;
  path: string | null;
  object_id: string | null;
  node_type: string | null;
  is_enabled: boolean;
  is_effectively_active: boolean;
  inactive_reason: string | null;
  show_inactive_chip: boolean;
  status_kind: string | null;
  collapse_children: boolean;
  warnings: string[];
  mod_count: number | null;
  children: PreviewTreeNode[];
};

export type CollectionPreview = {
  collection: CollectionSummary;
  members: CollectionMember[];
  mods: CollectionMod[];
  objects: CollectionObject[];
  roots: CollectionRoot[];
  tree_nodes: PreviewTreeNode[];
  projected_state: ProjectedCollectionState;
};

export type ApplyPreview = {
  collection_name: string;
  current_snapshot: string | null;
  current_mods: CollectionMod[];
  current_objects: CollectionObject[];
  current_tree_nodes: PreviewTreeNode[];
  target_mods: CollectionMod[];
  target_objects: CollectionObject[];
  target_tree_nodes: PreviewTreeNode[];
  current_state_name: string | null;
  current_state_is_unsaved: boolean;
  current_projected_state: ProjectedCollectionState;
  target_projected_state: ProjectedCollectionState;
};

export type ApplyResult = {
  success: boolean;
  mods_enabled: number;
  mods_disabled: number;
  objects_toggled: number;
  undo_collection_id: string | null;
  new_signature: string;
  warnings: string[];
  final_state_name: string | null;
  final_mode: string | null;
  partial_apply: boolean;
  skipped_missing_paths: string[];
  final_state_is_dirty: boolean;
  runtime_path_rewrites: WorkspacePathRewrite[];
};

export type CollectionPathRewrite = {
  from: string;
  to: string;
};

export type CollectionReferenceImpact = {
  affected_collection_count: number;
  affected_collection_names: string[];
  rewritten_paths: CollectionPathRewrite[];
  missing_paths: string[];
};

export type ProjectedObjectState = {
  object_id: string;
  display_name: string;
  path_key: string;
  is_enabled: boolean;
  active_root_count: number;
};

export type ProjectedActiveRoot = {
  object_id: string;
  root_key: string;
  display_name: string;
  root_type: string;
  source_path: string;
  thumbnail_hint: string | null;
  warnings: string[];
  is_missing: boolean;
};

export type ProjectedStateSummary = {
  object_count: number;
  enabled_object_count: number;
  active_root_count: number;
  missing_root_count: number;
};

export type ProjectedCollectionState = {
  object_states: ProjectedObjectState[];
  active_roots: ProjectedActiveRoot[];
  summary: ProjectedStateSummary;
};

export type ApplyProgressSnapshot = {
  game_id: string;
  is_safe: boolean;
  phase: string;
  completed: number;
  total: number;
  current_item: string | null;
  warnings: string[];
  final_state_name: string | null;
  final_mode: string | null;
  success: boolean;
};

export type PinStatus = {
  has_pin: boolean;
  is_locked: boolean;
  attempts_remaining: number;
  lockout_seconds_remaining: number;
};
