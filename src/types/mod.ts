/**
 * TypeScript types for mod folder operations.
 * Mirrors Rust structs from `folder_cmds.rs`, `trash.rs`, and `info_json.rs`.
 */

/** A single mod folder entry from `list_mod_folders` backend command. */
export interface ModFolder {
  /** Folder classification: ContainerFolder | ModPackRoot | VariantContainer | InternalAssets */
  node_type: string;
  /** Diagnostic reasons for the classification (debug/tooltip) */
  classification_reasons: string[];
  /** Database ID (UUID), if available */
  id?: string | null;
  /** Linked Object ID (foreign key) */
  object_id?: string | null;
  /** Display name (without "DISABLED " prefix) */
  name: string;
  /** Actual folder name on disk */
  folder_name: string;
  /** Full absolute path */
  path: string;
  /** Whether the mod is enabled (no "DISABLED " prefix) */
  is_enabled: boolean;
  /** Whether this entry is a directory */
  is_directory: boolean;
  /** Discovered thumbnail image path (if any) */
  thumbnail_path: string | null;
  /** Last modified time (epoch seconds) */
  modified_at: number;
  /** Total size in bytes */
  size_bytes: number;
  /** Whether the folder contains an info.json */
  has_info_json: boolean;
  /** Whether the mod is marked as favorite */
  is_favorite: boolean;
  /** Whether the mod appears to be in the wrong category */
  is_misplaced: boolean;
  /** Whether the mod is marked as safe (from info.json) */
  is_safe: boolean;
  /** Metadata from info.json (element, rarity, etc.) */
  metadata: Record<string, string> | null;
  /** Category from info.json metadata */
  category: string | null;
  /** Conflict group ID if this folder is part of a name collision */
  conflict_group_id?: string | null;
  /** Conflict state: "EnabledDisabledBothPresent" when both X and DISABLED X exist */
  conflict_state?: string | null;
}

/** Possible folder classification values from the backend classifier. */
export type NodeType =
  | 'ContainerFolder'
  | 'ModPackRoot'
  | 'VariantContainer'
  | 'InternalAssets'
  | 'FlatModRoot';

/** Response from the `list_mod_folders` backend command. */
export interface FolderGridResponse {
  self_node_type: string | null;
  self_is_mod: boolean;
  self_is_enabled: boolean;
  self_classification_reasons: string[];
  children: ModFolder[];
  /** Conflict groups detected in children (empty if none) */
  conflicts: ConflictGroup[];
}

/** A single member of a conflict group. */
export interface ConflictMember {
  path: string;
  folder_name: string;
  is_enabled: boolean;
  modified_at: number;
  size_bytes: number;
}

/** A group of folders sharing the same base name in the same parent directory. */
export interface ConflictGroup {
  group_id: string;
  base_name: string;
  members: ConflictMember[];
}

/** Only ContainerFolder allows double-click drill-down navigation. */
export function isNavigable(folder: ModFolder): boolean {
  return folder.node_type === 'ContainerFolder';
}

/** Mod metadata stored in `info.json` inside each mod folder. */
export interface ModInfo {
  actual_name: string;
  author: string;
  description: string;
  version: string;
  tags: string[];
  is_safe: boolean;
  is_auto_sync: boolean;
  is_favorite: boolean;
  metadata?: Record<string, string>;
}

/** Partial update for info.json — only provided fields are changed. */
export interface ModInfoUpdate {
  actual_name?: string;
  author?: string;
  description?: string;
  version?: string;
  tags?: string[];
  tags_add?: string[];
  tags_remove?: string[];
  is_safe?: boolean;
  is_auto_sync?: boolean;
  is_favorite?: boolean;
  metadata?: Record<string, string>;
}

/** Metadata for a trashed mod folder. */
export interface TrashEntry {
  id: string;
  original_path: string;
  original_name: string;
  deleted_at: string;
  size_bytes: number;
  game_id: string | null;
}

/** Result of a rename operation. */
export interface RenameResult {
  old_path: string;
  new_path: string;
  new_name: string;
}

/** Sort field for mod folder listing. */
export type SortField = 'name' | 'modified_at' | 'size_bytes';

/** Sort direction. */
export type SortOrder = 'asc' | 'desc';

/** Explorer view mode. */
export type ViewMode = 'grid' | 'list';

export interface BulkActionError {
  path: string;
  error: string;
}

export interface BulkResult {
  success: string[];
  failures: BulkActionError[];
}

/** Info about a shader/buffer hash conflict. */
export interface ConflictInfo {
  hash: string;
  section_name: string;
  /** Full paths of mods involved in this conflict */
  mod_paths: string[];
}

/** Info about a duplicate/conflicting enabled mod. */
export interface DuplicateInfo {
  mod_id: string;
  folder_path: string;
  actual_name: string;
}
