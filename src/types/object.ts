export type {
  CategoryCount,
  CategoryDef,
  JsonValue,
  ConflictGroup,
  ConflictMember,
  CreateObjectInput,
  CustomSkin,
  DbEntry,
  FilterDef,
  GameSchema,
  ModInfo,
  ObjectFilter,
  ObjectSummary,
  RenameResult,
} from '../lib/bindings.gen';

import type {
  ConflictGroup,
  ModInfoUpdate as GenModInfoUpdate,
  UpdateObjectInput as GenUpdateObjectInput,
} from '../lib/bindings.gen';

/**
 * All fields optional on purpose: every field is `Option` in Rust and serde
 * deserializes missing keys as `None`, so partial payloads are wire-valid.
 */
export type ModInfoUpdate = Partial<GenModInfoUpdate>;

/** Same wire semantics as ModInfoUpdate: every field is `Option` in Rust. */
export type UpdateObjectInput = Partial<GenUpdateObjectInput>;

export enum ObjectCategory {
  Character = 'Character',
  Weapon = 'Weapon',
  UI = 'UI',
  Other = 'Other',
}

export const OBJECT_CATEGORIES = Object.values(ObjectCategory);

/**
 * Numeric on the wire (Rust `serde_repr`): the generated bindings type this as
 * plain `number`; this enum is the frontend-side domain vocabulary for it.
 */
export enum ItemStatus {
  Disabled = 0,
  Enabled = 1,
}

import type { WorkspaceExplorerNode } from '../lib/bindings.gen';

/**
 * Frontend-only: the plain-folder subset of the generated
 * `WorkspaceExplorerNode` (Rust has no standalone struct for this shape).
 * Deriving via Omit keeps it structurally in sync with the wire type.
 */
export type ModFolder = Omit<
  WorkspaceExplorerNode,
  | 'node_kind'
  | 'display_mode'
  | 'type_chip'
  | 'display_name'
  | 'is_effectively_active'
  | 'ancestor_disabled'
  | 'inactive_reason'
  | 'warning_state'
  | 'primary_warning'
  | 'switch_state'
  | 'switch_reason'
  | 'switch_policy_key'
  | 'capabilities'
  | 'can_navigate'
>;

export type FolderGridResponse = {
  self_node_type: string | null;
  self_is_mod: boolean;
  self_is_enabled: boolean;
  self_owner_object_id?: string | null;
  self_owner_object_folder_path?: string | null;
  self_classification_reasons: string[];
  children: ModFolder[];
  conflicts: ConflictGroup[];
  /** Display name of the nearest disabled ancestor, if any. */
  ancestor_disabled_by?: string | null;
  /** Absolute path of the nearest disabled ancestor (for toggling). */
  ancestor_disabled_path?: string | null;
};
