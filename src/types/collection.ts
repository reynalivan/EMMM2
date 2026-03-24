// ---------------------------------------------------------------------------
// v2 Domain Types — Direct mirrors of Rust `domain/` structs
// ---------------------------------------------------------------------------

export type CollectionKind = 'named' | 'undo_snapshot' | 'unsaved';
export type MemberKind = 'mod' | 'nested' | 'object' | 'root';

export type SwitchResult = {
  success: boolean;
  mods_enabled: number;
  mods_disabled: number;
  objects_toggled: number;
};

export type CorridorSnapshot = {
  game_id: string;
  is_safe: boolean;
  signature?: string;
  mod_count?: number;
  timestamp?: string;
  active_collection_id: string | null;
  active_collection_name: string | null;
  is_dirty: boolean;
  undo_collection_id: string | null;
  current_signature: string;
  last_switched_at?: string | null;
  created_at?: string | null;
};

export type CorridorSwitchPreview = {
  game_id: string;
  source_safe: boolean;
  target_safe: boolean;
  target_signature: string | null;
  mods_to_enable: number;
  mods_to_disable: number;
  leaving_members: CollectionMember[];
  target_members: CollectionMember[];
  leaving_state_name: string;
  target_state_name: string;
  target_state_kind: string;
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

export type CollectionPreview = {
  collection: CollectionSummary;
  members: CollectionMember[];
  mods: CollectionMod[];
  objects: CollectionObject[];
  roots: CollectionRoot[];
};

export type ApplyPreview = {
  collection_name: string;
  current_snapshot: string | null;
  current_members: CollectionMember[];
  target_members: CollectionMember[];
  target_mods: CollectionMod[];
  target_objects: CollectionObject[];
};

export type ApplyResult = {
  success: boolean;
  mods_enabled: number;
  mods_disabled: number;
  objects_toggled: number;
  undo_collection_id: string | null;
  new_signature: string;
};

export type PinStatus = {
  has_pin: boolean;
  is_locked: boolean;
  attempts_remaining: number;
  lockout_seconds_remaining: number;
};
