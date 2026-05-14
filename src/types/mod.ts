/**
 * TypeScript types for mod folder operations.
 * Mirrors Rust structs from `folder_cmds.rs`, `trash.rs`, and `info_json.rs`.
 */

import type {
  ModFolder,
  FolderGridResponse,
  ConflictMember,
  ConflictGroup,
  ModInfo,
  ModInfoUpdate,
  RenameResult,
} from './object';
import type { BulkActionError, BulkResult, ConflictInfo, TrashMetadata } from './scanner';

/** A single mod folder entry from the WorkspaceViewModel explorer projection. */
export type { ModFolder };

/** Possible folder classification values from the backend classifier. */
export type NodeType =
  | 'ContainerFolder'
  | 'ModPackRoot'
  | 'VariantContainer'
  | 'InternalAssets'
  | 'FlatModRoot';

/** Response from the backend explorer projection. */
export type { FolderGridResponse };

/** A single member of a conflict group. */
export type { ConflictMember };

/** A group of folders sharing the same base name in the same parent directory. */
export type { ConflictGroup };

/** Only ContainerFolder allows double-click drill-down navigation. */
export function isNavigable(folder: ModFolder): boolean {
  return folder.node_type === 'ContainerFolder';
}

/** Mod metadata stored in `info.json` inside each mod folder. */
export type { ModInfo };

/** Partial update for info.json — only provided fields are changed. */
export type { ModInfoUpdate };

/** Metadata for a trashed mod folder. */
export type TrashEntry = TrashMetadata;

/** Result of a rename operation. */
export type { RenameResult };

/** Sort field for mod folder listing. */
export type SortField = 'name' | 'modified_at' | 'size_bytes';

/** Sort direction. */
export type SortOrder = 'asc' | 'desc';

/** Explorer view mode. */
export type ViewMode = 'grid' | 'list';

export type { BulkActionError };
export type { BulkResult };

/** Info about a shader/buffer hash conflict. */
export type { ConflictInfo };
