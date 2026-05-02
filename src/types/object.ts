/**
 * Types for Epic 3: Game Object Management & File Properties.
 * Directly mirrors Rust payload & test usages.
 */

export enum ObjectCategory {
  Character = 'Character',
  Weapon = 'Weapon',
  UI = 'UI',
  Other = 'Other',
}

export const OBJECT_CATEGORIES = Object.values(ObjectCategory);

export enum ItemStatus {
  Disabled = 0,
  Enabled = 1,
}

export interface CategoryDef {
  name: string;
  label?: string;
  icon: string;
  color: string;
  filters?: FilterDef[];
}

export interface FilterDef {
  key: string;
  label: string;
  options: string[];
}

export interface GameSchema {
  categories: CategoryDef[];
  filters: FilterDef[];
}

export type CategoryCount = {
  object_type: string;
  count: number;
};

export type ObjectSummary = {
  id: string;
  name: string;
  folder_path: string;
  matched_entry_key?: string | null;
  matched_alias_name?: string | null;
  matched_confidence?: number | null;
  matched_reason?: string | null;
  matched_source?: string | null;
  object_type: string;
  sub_category: string | null;
  status: number | null;
  created_at: string | null;
  mod_count: number;
  enabled_count: number;
  thumbnail_path: string | null;
  is_pinned: boolean;
  is_auto_sync: boolean;
  is_object_disabled: boolean;
  has_naming_conflict: boolean;
  metadata: string;
  tags: string;
  hash_db: Record<string, string[]> | null;
  custom_skins: Record<string, string> | null;
  /** Pipe-separated list of enabled mod paths for this object in the current corridor */
  active_mod_paths?: string | null;
};

export type ObjectFilter = {
  game_id: string;
  search_query: string | null;
  object_type: string | null;
  safe_mode: boolean;
  meta_filters: Record<string, string[]> | null;
  sort_by: string | null;
  status_filter: ItemStatus | null;
};

export type CreateObjectInput = {
  game_id: string;
  name: string;
  folder_path?: string | null;
  object_type: string;
  sub_category?: string | null;
  status?: ItemStatus | null;
  metadata?: unknown | null;
  thumbnail_url?: string | null;
  hash_db?: Record<string, string[]> | null;
  custom_skins?: Record<string, string> | null;
};

export type UpdateObjectInput = {
  name?: string | null;
  object_type?: string | null;
  sub_category?: string | null;
  status?: ItemStatus | null;
  metadata?: unknown | null;
  hash_db?: Record<string, string[]> | null;
  custom_skins?: Record<string, string> | null;
  thumbnail_path?: string | null;
  is_auto_sync?: boolean | null;
  tags?: string[] | null;
};

export type CustomSkin = {
  name: string;
  aliases: string[];
  thumbnail_skin_path: string | null;
  rarity: string | null;
};

export type DbEntry = {
  name: string;
  tags: string[];
  object_type: string;
  custom_skins: CustomSkin[];
  thumbnail_path: string | null;
  metadata: Record<string, unknown> | null;
  hash_db: Record<string, string[]>;
};

export type ModFolder = {
  node_type: string;
  classification_reasons: string[];
  id?: string | null;
  owner_object_id?: string | null;
  owner_object_folder_path?: string | null;
  name: string;
  folder_name: string;
  path: string;
  is_enabled: boolean;
  is_directory: boolean;
  thumbnail_path: string | null;
  modified_at: number;
  size_bytes: number;
  has_info_json: boolean;
  is_favorite: boolean;
  is_misplaced: boolean;
  is_safe: boolean;
  metadata: Record<string, string> | null;
  category: string | null;
  conflict_group_id?: string | null;
  conflict_state?: string | null;
  pin_hash?: string | null;
  warnings: string[];
};

export type ModInfo = {
  actual_name: string;
  author: string;
  description: string;
  version: string;
  tags: string[];
  is_safe: boolean;
  is_favorite: boolean;
  is_auto_sync: boolean;
  preset_name: string[];
  metadata: Record<string, string>;
};

export type ModInfoUpdate = {
  actual_name?: string | null;
  author?: string | null;
  description?: string | null;
  version?: string | null;
  tags?: string[] | null;
  tags_add?: string[] | null;
  tags_remove?: string[] | null;
  is_safe?: boolean | null;
  is_favorite?: boolean | null;
  is_auto_sync?: boolean | null;
  preset_name_add?: string[] | null;
  preset_name_remove?: string[] | null;
  metadata?: Record<string, string> | null;
};

export type ConflictMember = {
  path: string;
  folder_name: string;
  is_enabled: boolean;
  modified_at: number;
  size_bytes: number;
};

export type ConflictGroup = {
  group_id: string;
  base_name: string;
  members: ConflictMember[];
};

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

export type RenameResult = {
  old_path: string;
  new_path: string;
  new_name: string;
};

export interface GetObjectsResult {
  objects: ObjectSummary[];
  lost_objects: string[];
}

export interface FolderContentInfo {
  path: string;
  name: string;
  item_count: number;
  is_empty: boolean;
}
